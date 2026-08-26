use github_personal_stats_core::{
    ACTIVITY_SCHEMA, ActivitySnapshot, Author, DayBucket, LineCounts, MEASURE_AGENT, TimeBucket,
    UNKNOWN_LANGUAGE, merge_snapshots, parse_activity_snapshot, summarise_activity,
    write_activity_snapshot,
};

/// A day whose time is agent time, which is what the editor's own record can
/// tell us. Editor time arrives separately, from the plugins.
fn day(date: &str, seconds: u64) -> DayBucket {
    let mut bucket = DayBucket::new(date);
    bucket.measure_mut("agent").seconds = seconds;
    bucket
}

fn snapshot(machine: &str, collected_at: &str, days: Vec<DayBucket>) -> ActivitySnapshot {
    let mut snapshot = ActivitySnapshot::new(machine, collected_at);
    snapshot.days = days;
    snapshot
}

/// Sums a day's facts for one model, whatever languages they fell in.
fn lines_by_model(day: &DayBucket, model: &str) -> u64 {
    day.lines
        .iter()
        .filter(|fact| fact.model == model)
        .map(|fact| fact.total())
        .sum()
}

/// Sums a day's facts for one author.
fn lines_by_author(day: &DayBucket, author: Author) -> u64 {
    day.lines
        .iter()
        .filter(|fact| fact.author == author)
        .map(|fact| fact.total())
        .sum()
}

#[test]
fn two_machines_working_the_same_day_add_up() {
    let laptop = snapshot(
        "m-laptop",
        "2026-08-24T19:00:00Z",
        vec![day("2026-08-24", 3600)],
    );
    let desktop = snapshot(
        "m-desktop",
        "2026-08-24T19:05:00Z",
        vec![day("2026-08-24", 1800)],
    );

    let merged = merge_snapshots(&[laptop, desktop]);

    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].measure("agent").seconds, 5400);
}

#[test]
fn collecting_twice_on_one_machine_still_counts_the_day_once() {
    let first = snapshot(
        "m-laptop",
        "2026-08-24T12:00:00Z",
        vec![day("2026-08-24", 3600)],
    );
    let second = snapshot(
        "m-laptop",
        "2026-08-24T19:00:00Z",
        vec![day("2026-08-24", 5400)],
    );

    let merged = merge_snapshots(&[first, second]);

    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].measure("agent").seconds, 5400);
}

#[test]
fn a_stale_copy_of_a_machine_loses_to_the_fresher_one_whichever_order_it_arrives() {
    let stale = snapshot(
        "m-laptop",
        "2026-08-01T09:00:00Z",
        vec![day("2026-08-24", 60)],
    );
    let fresh = snapshot(
        "m-laptop",
        "2026-08-24T19:00:00Z",
        vec![day("2026-08-24", 7200)],
    );

    let forwards = merge_snapshots(&[stale.clone(), fresh.clone()]);
    let backwards = merge_snapshots(&[fresh, stale]);

    assert_eq!(forwards[0].measure("agent").seconds, 7200);
    assert_eq!(backwards[0].measure("agent").seconds, 7200);
}

#[test]
fn merged_days_come_back_in_date_order() {
    let one = snapshot(
        "m-laptop",
        "2026-08-24T19:00:00Z",
        vec![day("2026-08-24", 10), day("2026-08-22", 10)],
    );
    let two = snapshot(
        "m-desktop",
        "2026-08-24T19:00:00Z",
        vec![day("2026-08-23", 10)],
    );

    let merged = merge_snapshots(&[one, two]);

    let dates = merged
        .iter()
        .map(|bucket| bucket.date.as_str())
        .collect::<Vec<_>>();
    assert_eq!(dates, ["2026-08-22", "2026-08-23", "2026-08-24"]);
}

#[test]
fn languages_models_and_lines_all_add_up_across_machines() {
    let mut morning = day("2026-08-24", 600);
    morning
        .measure_mut("agent")
        .languages
        .insert("Rust".to_string(), 600);
    morning.add_lines("", Author::Agent, "composer-2.5", 40, 0);
    morning.commits.agent_added = 40;

    let mut evening = day("2026-08-24", 300);
    evening
        .measure_mut("agent")
        .languages
        .insert("Rust".to_string(), 100);
    evening
        .measure_mut("agent")
        .languages
        .insert("Markdown".to_string(), 200);
    evening.add_lines("", Author::Agent, "composer-2.5", 10, 0);
    evening.add_lines("", Author::Human, "", 5, 0);
    evening.commits.human_added = 5;

    let merged = merge_snapshots(&[
        snapshot("m-laptop", "2026-08-24T19:00:00Z", vec![morning]),
        snapshot("m-desktop", "2026-08-24T19:00:00Z", vec![evening]),
    ]);

    assert_eq!(merged[0].measure("agent").languages["Rust"], 700);
    assert_eq!(merged[0].measure("agent").languages["Markdown"], 200);
    assert_eq!(lines_by_model(&merged[0], "composer-2.5"), 50);
    assert_eq!(lines_by_author(&merged[0], Author::Human), 5);
    assert_eq!(merged[0].commits.agent_added, 40);
    assert_eq!(merged[0].commits.human_added, 5);
}

#[test]
fn committed_lines_and_written_lines_stay_separate_measures() {
    let mut bucket = day("2026-08-24", 0);
    bucket.commits.agent_added = 23;
    bucket.commits.human_added = 0;
    bucket.add_lines("", Author::Agent, "gpt-5.5", 900, 0);
    bucket.add_lines("", Author::Human, "", 100, 0);

    let totals = summarise_activity(&[bucket]);

    assert_eq!(totals.commits.added(), 23);
    assert_eq!(totals.lines.total(), 1000);
    assert_eq!(totals.lines.ai_share_basis_points(), 9000);
    assert_eq!(totals.commits.ai_share_basis_points(), 10_000);
}

#[test]
fn a_snapshot_reads_back_as_what_was_written() {
    let mut bucket = day("2026-08-24", 5400);
    bucket
        .measure_mut("agent")
        .languages
        .insert("Rust".to_string(), 5000);
    bucket.add_lines("", Author::Agent, "claude-opus-5", 812, 0);
    bucket.add_lines("", Author::Human, "", 8, 0);
    bucket.commits = LineCounts {
        agent_added: 812,
        agent_deleted: 91,
        human_added: 8,
        human_deleted: 1,
        blank_added: 40,
        unattributed_added: 120,
        ..LineCounts::default()
    };
    bucket.measure_mut("agent").sessions = 3;
    bucket.requests = 24;

    let mut original = snapshot("m-8f3a", "2026-08-24T19:00:00Z", vec![bucket]);
    original
        .cursors
        .insert("cursor_scored".to_string(), "1787309470".to_string());

    let written = write_activity_snapshot(&original).expect("snapshot should write");
    let read_back = parse_activity_snapshot(&written).expect("snapshot should read");

    assert_eq!(read_back, original);
}

#[test]
fn a_snapshot_from_a_future_build_is_refused_rather_than_half_read() {
    let mut ahead = snapshot("m-8f3a", "2026-08-24T19:00:00Z", Vec::new());
    ahead.schema = ACTIVITY_SCHEMA + 1;

    let written = serde_json::to_string(&ahead).expect("test snapshot should serialise");
    let error = parse_activity_snapshot(&written).expect_err("a newer schema should be refused");

    assert!(error.to_string().contains("newer than this build"));
}

#[test]
fn a_machine_id_that_could_escape_its_filename_is_refused() {
    for hostile in ["../../etc/passwd", "Liu-MacBook", "has space", ""] {
        let snapshot = snapshot(hostile, "2026-08-24T19:00:00Z", Vec::new());
        assert!(
            write_activity_snapshot(&snapshot).is_err(),
            "machine id {hostile:?} should be refused"
        );
    }
}

#[test]
fn a_timestamp_without_a_zone_is_refused_because_merging_orders_by_it() {
    let snapshot = snapshot("m-8f3a", "2026-08-24 19:00:00", Vec::new());

    let error =
        write_activity_snapshot(&snapshot).expect_err("a local timestamp should be refused");

    assert!(error.to_string().contains("2026-08-24T19:00:00Z"));
}

#[test]
fn totals_rank_languages_and_models_by_size() {
    let mut first = day("2026-08-23", 1200);
    first
        .measure_mut("agent")
        .languages
        .insert("Rust".to_string(), 900);
    first
        .measure_mut("agent")
        .languages
        .insert("Shell".to_string(), 300);
    first.add_lines("", Author::Agent, "composer-2.5", 100, 0);

    let mut second = day("2026-08-24", 600);
    second
        .measure_mut("agent")
        .languages
        .insert("Markdown".to_string(), 600);
    second.add_lines("", Author::Agent, "claude-opus-5", 400, 0);

    let totals = summarise_activity(&[first, second]);

    assert_eq!(totals.measure("agent").seconds, 1800);
    assert_eq!(totals.active_days, 2);
    assert_eq!(totals.first_day.as_deref(), Some("2026-08-23"));
    assert_eq!(totals.last_day.as_deref(), Some("2026-08-24"));
    assert_eq!(totals.measure("agent").languages[0].language, "Rust");
    assert_eq!(totals.measure("agent").languages[0].seconds, 900);
    assert_eq!(totals.measure("agent").languages[1].language, "Markdown");
    assert_eq!(totals.models()[0].name, "claude-opus-5");
}

#[test]
fn ai_share_weighs_agent_and_tab_against_the_lines_that_could_be_attributed() {
    let counts = LineCounts {
        agent_added: 70,
        tab_added: 10,
        human_added: 20,
        blank_added: 500,
        unattributed_added: 400,
        ..LineCounts::default()
    };

    assert_eq!(counts.ai_added(), 80);
    assert_eq!(counts.attributed(), 100);
    assert_eq!(counts.added(), 1000);
    assert_eq!(counts.ai_share_basis_points(), 8000);
    assert_eq!(counts.attributed_share_basis_points(), 1000);
}

#[test]
fn lines_nobody_could_attribute_never_land_in_the_ai_share() {
    let counts = LineCounts {
        agent_added: 10,
        human_added: 10,
        unattributed_added: 9_000,
        ..LineCounts::default()
    };

    assert_eq!(counts.ai_share_basis_points(), 5000);
}

#[test]
fn a_day_nobody_worked_reports_no_share_instead_of_dividing_by_zero() {
    assert_eq!(LineCounts::default().ai_share_basis_points(), 0);
}

#[test]
fn editor_time_and_agent_time_are_kept_apart_because_they_measure_different_things() {
    let mut bucket = DayBucket::new("2026-08-24");
    bucket.measure_mut("editor").seconds = 20 * 3600;
    bucket.measure_mut("editor").sessions = 4;
    bucket
        .measure_mut("editor")
        .languages
        .insert("Markdown".to_string(), 20 * 3600);
    bucket.measure_mut("agent").seconds = 3 * 3600;
    bucket.measure_mut("agent").sessions = 9;
    bucket
        .measure_mut("agent")
        .languages
        .insert("Rust".to_string(), 3 * 3600);

    let merged = merge_snapshots(&[snapshot("m-laptop", "2026-08-24T19:00:00Z", vec![bucket])]);
    let totals = summarise_activity(&merged);

    assert_eq!(totals.measure("editor").seconds, 20 * 3600);
    assert_eq!(totals.measure("agent").seconds, 3 * 3600);
    assert_eq!(totals.measure("editor").sessions, 4);
    assert_eq!(totals.measure("agent").sessions, 9);
    assert_eq!(totals.measure("editor").languages[0].language, "Markdown");
    assert_eq!(totals.measure("agent").languages[0].language, "Rust");
}

#[test]
fn a_day_with_only_editor_time_still_counts_as_worked() {
    let mut bucket = DayBucket::new("2026-08-24");
    bucket.measure_mut("editor").seconds = 1800;

    let totals = summarise_activity(&[bucket]);

    assert_eq!(totals.active_days, 1);
    assert_eq!(totals.measure("agent").seconds, 0);
}

#[test]
fn a_named_language_supersedes_the_unnamed_reading_of_the_same_lines() {
    // A day recorded before languages were kept says only that a model wrote so
    // many lines. Reading the same day again, with languages, describes those
    // same lines more finely, and the two readings must not be added.
    let mut held = DayBucket::new("2026-08-21");
    held.add_lines(UNKNOWN_LANGUAGE, Author::Agent, "claude-opus-5", 22_472, 0);

    let mut fresh = DayBucket::new("2026-08-21");
    fresh.add_lines("Rust", Author::Agent, "claude-opus-5", 20_000, 0);
    fresh.add_lines("Markdown", Author::Agent, "claude-opus-5", 2_436, 0);
    fresh.add_lines(UNKNOWN_LANGUAGE, Author::Agent, "claude-opus-5", 36, 0);

    held.keep_fuller(&fresh);
    let written = |language: &str| {
        held.lines
            .iter()
            .filter(|fact| fact.language == language)
            .map(|fact| fact.added)
            .sum::<u64>()
    };

    assert_eq!(written("Rust"), 20_000);
    assert_eq!(written("Markdown"), 2_436);
    // What the coarse reading claimed beyond the named languages is the part of
    // it that survives: files whose extension nobody recorded.
    assert_eq!(written(UNKNOWN_LANGUAGE), 36);
    assert_eq!(
        held.lines.iter().map(|fact| fact.added).sum::<u64>(),
        22_472
    );
}

#[test]
fn an_unnamed_reading_with_nothing_finer_beside_it_is_kept_whole() {
    // The source a day came from keeps only a few weeks, so an old day may only
    // ever have the coarse reading. Superseding it with nothing would erase it.
    let mut held = DayBucket::new("2026-07-26");
    held.add_lines(UNKNOWN_LANGUAGE, Author::Agent, "gpt-5.5", 770, 0);
    held.keep_fuller(&DayBucket::new("2026-07-26"));

    assert_eq!(held.lines.len(), 1);
    assert_eq!(held.lines[0].added, 770);
}

#[test]
fn time_divides_by_author_without_exceeding_what_it_divides() {
    let mut bucket = TimeBucket {
        seconds: 100,
        ..TimeBucket::default()
    };
    bucket.languages.insert("Rust".to_owned(), 100);
    bucket.spend("Rust", Author::Agent, 70);
    bucket.spend("Rust", Author::Human, 20);

    assert_eq!(bucket.attributed("Rust", Author::Agent), 70);
    assert_eq!(bucket.attributed("Rust", Author::Human), 20);
    // The ten seconds nobody could put a name to stay unattributed rather than
    // being handed to whichever author was handy.
    let named = bucket
        .by_author
        .iter()
        .map(|fact| fact.seconds)
        .sum::<u64>();
    assert_eq!(named, 90);
    assert!(named <= bucket.seconds);
}

#[test]
fn a_source_that_cannot_name_an_author_leaves_the_split_empty() {
    // An imported day knows how long a language took and nothing about who was
    // doing it, so it must not appear to claim that nobody was.
    let mut bucket = TimeBucket {
        seconds: 3_600,
        ..TimeBucket::default()
    };
    bucket.languages.insert("Clojure".to_owned(), 3_600);

    assert!(bucket.by_author.is_empty());
    assert_eq!(bucket.attributed("Clojure", Author::Agent), 0);
}

#[test]
fn two_machines_add_their_attributed_time_and_two_readings_do_not() {
    let day = |seconds: u64| {
        let mut day = DayBucket::new("2026-08-20");
        let bucket = day.measure_mut(MEASURE_AGENT);
        bucket.seconds = seconds;
        bucket.spend("Go", Author::Agent, seconds);
        day
    };

    // Two machines worked an hour and half an hour on the same day, which is an
    // hour and a half of work. Summing happens as a record is read, so it is
    // checked through the summary rather than reached into.
    let summed = summarise_activity(&[day(3_600), day(1_800)]);
    assert_eq!(summed.measure(MEASURE_AGENT).seconds, 5_400);

    // One machine read twice saw the same hour twice, which is still an hour.
    let mut merged = day(3_600);
    merged.keep_fuller(&day(1_800));
    assert_eq!(
        merged
            .measure(MEASURE_AGENT)
            .attributed("Go", Author::Agent),
        3_600
    );
}
