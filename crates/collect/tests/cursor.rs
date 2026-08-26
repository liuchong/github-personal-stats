//! Reading Cursor's own record of what it wrote.
//!
//! This is the one place the collector touches a database it does not own, whose
//! shape can change under it and whose rows are not always well behaved. The
//! tests build a database with that schema and put awkward rows in it, because
//! the alternative is finding out from a wrong number on a card.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use github_personal_stats_collect::{
    cursor::{commit_day, database_path, read as read_source},
    error::CollectError,
    sessions,
};
use github_personal_stats_core::{Author, DayBucket, MEASURE_AGENT};
use rusqlite::Connection;

/// Reads this source and turns its moments into hours.
///
/// The reader hands out moments rather than hours because hours are not a
/// property of one source: the collector sorts every source's moments into one
/// timeline so that two agents working the same afternoon are one afternoon. What
/// these tests check is this source's own reading, so they do that last step
/// themselves.
fn read(
    path: &Path,
    idle_timeout_seconds: i64,
) -> Result<BTreeMap<String, DayBucket>, CollectError> {
    let reading = read_source(path)?;
    let mut days = reading.days;
    for (date, worked) in sessions::accumulate(&reading.moments, idle_timeout_seconds) {
        *days
            .entry(date.clone())
            .or_insert_with(|| DayBucket::new(&date))
            .measure_mut(MEASURE_AGENT) = worked;
    }
    Ok(days)
}

fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("gps-cursor-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("a scratch directory");
    root
}

/// A database with the schema this reader expects. Only the columns it selects
/// are declared, which is also a check that it selects no more than these.
fn database(root: &Path) -> PathBuf {
    let path = root.join("ai-code-tracking.db");
    let connection = Connection::open(&path).expect("a database to write");
    connection
        .execute_batch(
            "CREATE TABLE scored_commits (
                commitHash TEXT,
                commitDate TEXT,
                tabLinesAdded INTEGER, tabLinesDeleted INTEGER,
                composerLinesAdded INTEGER, composerLinesDeleted INTEGER,
                humanLinesAdded INTEGER, humanLinesDeleted INTEGER,
                blankLinesAdded INTEGER, blankLinesDeleted INTEGER,
                linesAdded INTEGER, linesDeleted INTEGER
             );
             CREATE TABLE ai_code_hashes (
                createdAt INTEGER,
                source TEXT,
                model TEXT,
                fileExtension TEXT,
                requestId TEXT
             );",
        )
        .expect("the schema should be created");
    path
}

/// The shape git prints, which is what Cursor stores.
const GIT_DATE: &str = "Mon Aug 24 19:00:00 2026 +0800";

fn commit(path: &Path, hash: &str, date: &str, counts: [i64; 10]) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute(
            "INSERT INTO scored_commits VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                hash, date, counts[0], counts[1], counts[2], counts[3], counts[4], counts[5],
                counts[6], counts[7], counts[8], counts[9],
            ],
        )
        .expect("a commit row should insert");
}

fn hash(path: &Path, at_seconds: i64, source: &str, model: &str, ext: &str, request: &str) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute(
            "INSERT INTO ai_code_hashes VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![at_seconds * 1_000, source, model, ext, request],
        )
        .expect("a hash row should insert");
}

/// Midnight UTC on 24 August 2026, plus an offset in seconds.
fn at(offset: i64) -> i64 {
    1_787_529_600 + offset
}

/// Sums the lines one author wrote with one model, across languages.
fn written(day: &DayBucket, author: Author, model: &str) -> u64 {
    day.lines
        .iter()
        .filter(|fact| fact.author == author && fact.model == model)
        .map(|fact| fact.total())
        .sum()
}

#[test]
fn a_database_that_is_not_there_is_reported_as_missing_rather_than_as_empty() {
    let root = scratch("absent");

    let outcome = read(&root.join("nothing.db"), 300);

    assert!(
        outcome.is_err(),
        "an absent database is a different thing from a quiet one"
    );
}

#[test]
fn committed_lines_are_split_by_who_wrote_them() {
    let root = scratch("committed");
    let path = database(&root);
    // 40 agent, 10 tab, 20 human, 5 blank, of 100 added in total.
    commit(&path, "abc", GIT_DATE, [10, 1, 40, 4, 20, 2, 5, 0, 100, 10]);

    let days = read(&path, 300).expect("a readable database");
    let day = days.get("2026-08-24").expect("the commit's day");

    assert_eq!(day.commits.agent_added, 40);
    assert_eq!(day.commits.tab_added, 10);
    assert_eq!(day.commits.human_added, 20);
    assert_eq!(day.commits.blank_added, 5);
    // Whatever the total does not account for is not quietly assigned to anyone.
    assert_eq!(day.commits.unattributed_added, 25);
    assert_eq!(day.commits.unattributed_deleted, 3);
}

#[test]
fn a_commit_counted_twice_is_only_counted_once() {
    // The table has a row per file, so a commit touching three files appears
    // three times with the same totals.
    let root = scratch("duplicates");
    let path = database(&root);
    for _ in 0..3 {
        commit(&path, "same", GIT_DATE, [0, 0, 30, 0, 0, 0, 0, 0, 30, 0]);
    }

    let days = read(&path, 300).unwrap();

    assert_eq!(days["2026-08-24"].commits.agent_added, 30);
}

#[test]
fn a_total_smaller_than_its_parts_does_not_wrap_around() {
    // Saturating rather than underflowing, which on unsigned counts would turn a
    // small inconsistency into billions of lines.
    let root = scratch("inconsistent");
    let path = database(&root);
    commit(&path, "odd", GIT_DATE, [0, 0, 90, 0, 0, 0, 0, 0, 10, 0]);

    let days = read(&path, 300).unwrap();

    assert_eq!(days["2026-08-24"].commits.unattributed_added, 0);
    assert_eq!(days["2026-08-24"].commits.agent_added, 90);
}

#[test]
fn negative_counts_are_not_believed() {
    let root = scratch("negative");
    let path = database(&root);
    commit(&path, "neg", GIT_DATE, [0, 0, -5, 0, 7, 0, 0, 0, 7, 0]);

    let days = read(&path, 300).unwrap();

    assert_eq!(days["2026-08-24"].commits.agent_added, 0);
    assert_eq!(days["2026-08-24"].commits.human_added, 7);
}

#[test]
fn a_commit_with_no_date_is_skipped_rather_than_dated_today() {
    let root = scratch("undated");
    let path = database(&root);
    commit(&path, "dated", GIT_DATE, [0, 0, 5, 0, 0, 0, 0, 0, 5, 0]);
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "INSERT INTO scored_commits (commitHash, commitDate, composerLinesAdded, linesAdded) \
             VALUES ('undated', NULL, 999, 999)",
            [],
        )
        .unwrap();

    let days = read(&path, 300).unwrap();

    assert_eq!(days.len(), 1, "only the dated commit should land");
    assert_eq!(days["2026-08-24"].commits.agent_added, 5);
}

#[test]
fn generated_lines_are_counted_per_model_and_humans_kept_apart() {
    let root = scratch("generated");
    let path = database(&root);
    for _ in 0..7 {
        hash(&path, at(0), "composer", "claude-opus", "rs", "r1");
    }
    for _ in 0..3 {
        hash(&path, at(0), "composer", "gpt-5", "rs", "r2");
    }
    for _ in 0..2 {
        hash(&path, at(0), "human", "", "rs", "r3");
    }

    let days = read(&path, 300).unwrap();
    let day = days.values().next().expect("a day");

    assert_eq!(written(day, Author::Agent, "claude-opus"), 7);
    assert_eq!(written(day, Author::Agent, "gpt-5"), 3);
    assert_eq!(written(day, Author::Human, ""), 2);
}

#[test]
fn a_model_that_did_not_name_itself_is_recorded_as_unknown() {
    let root = scratch("unnamed");
    let path = database(&root);
    hash(&path, at(0), "composer", "", "rs", "r1");

    let days = read(&path, 300).unwrap();
    let day = days.values().next().unwrap();

    // A model that did not name itself leaves the model empty rather than
    // inventing a name, so the lines are still counted as an agent's.
    assert_eq!(written(day, Author::Agent, ""), 1);
}

#[test]
fn work_close_together_is_one_sitting_and_a_gap_starts_another() {
    let root = scratch("sessions");
    let path = database(&root);
    // Three minutes of work, then a twenty minute gap, then two more minutes.
    for minute in [0, 1, 2, 3] {
        hash(&path, at(minute * 60), "composer", "m", "rs", "r1");
    }
    for minute in [23, 24] {
        hash(&path, at(minute * 60), "composer", "m", "rs", "r2");
    }

    let days = read(&path, 300).unwrap();
    let day = days.values().next().unwrap();

    assert_eq!(
        day.measure("agent").sessions,
        2,
        "a twenty minute gap ends a sitting"
    );
    // Each sitting is measured from its first moment to its last, so the gap
    // itself is not counted: three minutes plus one.
    assert_eq!(day.measure("agent").seconds, 240);
}

#[test]
fn a_longer_idle_timeout_joins_what_a_shorter_one_separates() {
    let root = scratch("timeout");
    let path = database(&root);
    for minute in [0, 10] {
        hash(&path, at(minute * 60), "composer", "m", "rs", "r1");
    }

    let tight = read(&path, 300).unwrap();
    let loose = read(&path, 1_200).unwrap();

    assert_eq!(tight.values().next().unwrap().measure("agent").sessions, 2);
    assert_eq!(loose.values().next().unwrap().measure("agent").sessions, 1);
}

#[test]
fn time_is_attributed_to_the_language_being_worked_on_at_the_time() {
    // Each stretch between two moments counts towards the file that was being
    // written when it started, so switching language part way through a sitting
    // splits the time rather than assigning all of it to one.
    let root = scratch("languages");
    let path = database(&root);
    for second in [0, 60] {
        hash(&path, at(second), "composer", "m", "rs", "r1");
    }
    for second in [120, 180] {
        hash(&path, at(second), "composer", "m", "md", "r1");
    }

    let days = read(&path, 300).unwrap();
    let languages = &days.values().next().unwrap().measure("agent").languages;

    assert_eq!(languages.get("Rust"), Some(&120), "{languages:?}");
    assert_eq!(languages.get("Markdown"), Some(&60), "{languages:?}");
}

#[test]
fn the_moment_that_closes_a_sitting_does_not_claim_time_of_its_own() {
    // It ends the last stretch rather than beginning one, so a single touch of a
    // file at the very end is not credited with minutes it did not take.
    let root = scratch("closing");
    let path = database(&root);
    hash(&path, at(0), "composer", "m", "rs", "r1");
    hash(&path, at(60), "composer", "m", "rs", "r1");
    hash(&path, at(120), "composer", "m", "md", "r1");

    let days = read(&path, 300).unwrap();
    let languages = &days.values().next().unwrap().measure("agent").languages;

    assert_eq!(languages.get("Rust"), Some(&120), "{languages:?}");
    assert_eq!(languages.get("Markdown"), None, "{languages:?}");
}

#[test]
fn a_file_with_no_extension_does_not_lose_the_time_spent_on_it() {
    let root = scratch("no-extension");
    let path = database(&root);
    hash(&path, at(0), "composer", "m", "", "r1");
    hash(&path, at(60), "composer", "m", "", "r1");

    let days = read(&path, 300).unwrap();
    let day = days.values().next().unwrap();

    assert!(
        day.measure("agent").seconds > 0,
        "the time was still worked"
    );
}

#[test]
fn requests_are_counted_once_each_however_many_lines_they_produced() {
    let root = scratch("requests");
    let path = database(&root);
    for _ in 0..20 {
        hash(&path, at(0), "composer", "m", "rs", "r1");
    }
    hash(&path, at(30), "composer", "m", "rs", "r2");

    let days = read(&path, 300).unwrap();

    assert_eq!(days.values().next().unwrap().requests, 2);
}

#[test]
fn a_quiet_database_reads_as_no_days_rather_than_as_a_failure() {
    let root = scratch("quiet");
    let path = database(&root);

    let days = read(&path, 300).expect("an empty database is readable");

    assert!(days.is_empty());
}

#[test]
fn a_database_missing_the_tables_says_which_one_it_could_not_read() {
    let root = scratch("wrong-shape");
    let path = root.join("empty.db");
    Connection::open(&path).unwrap();

    let outcome = read(&path, 300);

    let message = outcome.expect_err("a database without the tables cannot be read");
    assert!(
        format!("{message}").contains("scored_commits"),
        "it should name the table: {message}"
    );
}

#[test]
fn a_commit_date_is_read_from_the_shape_git_prints() {
    assert_eq!(commit_day(GIT_DATE), Some("2026-08-24".to_owned()));
    assert_eq!(
        commit_day("Sun Jan 5 08:30:00 2025 -0500"),
        Some("2025-01-05".to_owned())
    );
}

#[test]
fn a_date_that_makes_no_sense_is_declined_rather_than_guessed() {
    assert_eq!(commit_day(""), None);
    assert_eq!(commit_day("not a date"), None);
    // An ISO date is not the shape this column holds, and guessing at it would
    // silently date commits wrongly.
    assert_eq!(commit_day("2026-08-24"), None);
    assert_eq!(commit_day("Mon Smarch 24 19:00:00 2026 +0800"), None);
    assert_eq!(commit_day("Mon Aug 99 19:00:00 2026 +0800"), None);
    assert_eq!(commit_day("Mon Aug 24 19:00:00 1900 +0800"), None);
}

/// The editor writes to this database while the collector reads it, and keeps it
/// on a rollback journal, so a write in progress shuts readers out completely.
/// A read that gave up on the lock would lose the whole run, so it has to wait
/// for the owner to finish instead.
#[test]
fn a_read_waits_for_the_editor_to_finish_writing() {
    let root = scratch("locked");
    let path = database(&root);
    hash(&path, 1_756_000_000, "composer", "a-model", "rs", "r1");

    let writer = Connection::open(&path).expect("a writer");
    writer
        .execute_batch("BEGIN EXCLUSIVE")
        .expect("the writer should take the lock");

    let holding = std::time::Duration::from_millis(400);
    let released = std::thread::spawn(move || {
        std::thread::sleep(holding);
        writer
            .execute_batch("COMMIT")
            .expect("the writer should let go");
    });

    let started = std::time::Instant::now();
    let days = read(&path, 300).expect("a read should wait rather than fail");
    let waited = started.elapsed();
    released.join().expect("the writer thread should finish");

    assert_eq!(days.len(), 1, "the row should be read once the lock clears");
    assert!(
        waited >= holding,
        "the read returned in {waited:?}, so it cannot have waited for the lock"
    );
}

/// Waiting out a busy owner and repairing a changed schema are opposite jobs, so
/// the message has to say which one happened. It previously reported a held lock
/// as `scored_commits does not look the way this build expects`, which points at
/// a column that was never missing.
#[test]
fn a_held_lock_does_not_read_as_a_broken_schema() {
    let busy = CollectError::Busy {
        what: "scored_commits",
        waited: std::time::Duration::from_secs(30),
    }
    .to_string();

    assert!(busy.contains("locked"), "{busy}");
    assert!(busy.contains("30"), "{busy}");
    assert!(
        !busy.contains("does not look the way"),
        "a lock is not a schema change: {busy}"
    );
}

#[test]
fn the_database_is_looked_for_where_cursor_keeps_it() {
    let path = database_path(&PathBuf::from("/home/someone"));

    assert!(path.ends_with("ai-code-tracking.db"), "{path:?}");
    assert!(path.to_string_lossy().contains(".cursor"), "{path:?}");
}
