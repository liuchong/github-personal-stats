//! How a collection becomes a record without losing what came before.
//!
//! The sources a collection reads forget after about a month. These tests are
//! mostly about what happens on the run after that: whether the record still
//! holds the day, and whether a run that learned nothing new leaves the files
//! alone so a sink on a timer has nothing to commit.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};

use github_personal_stats_collect::records::{publish, read_days};
use github_personal_stats_core::{
    ActivitySnapshot, Author, DayBucket,
    store::{MANIFEST, parse_manifest},
};

fn worked(date: &str, agent_seconds: u64) -> DayBucket {
    let mut day = DayBucket::new(date);
    day.measure_mut("agent").seconds = agent_seconds;
    day.measure_mut("agent").sessions = 2;
    day.measure_mut("agent")
        .languages
        .insert("Rust".to_owned(), agent_seconds / 2);
    day.commits.agent_added = agent_seconds / 10;
    day.add_lines("", Author::Agent, "a-model", 500, 0);
    day.requests = 7;
    day
}

fn reading(collected_at: &str, days: Vec<DayBucket>) -> ActivitySnapshot {
    let mut snapshot = ActivitySnapshot::new("m-laptop", collected_at);
    snapshot.days = days;
    snapshot
}

/// A root of its own for each test. These tests run at the same time and each
/// one publishes as the same machine, so a shared directory would have them
/// reading each other's days.
fn root() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let directory = std::env::temp_dir().join(format!(
        "gps-records-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a scratch directory should be creatable");
    directory
}

#[test]
fn a_day_becomes_its_own_file() {
    let place = root();

    let written = publish(
        place.as_path(),
        &reading(
            "2026-08-26T00:00:00Z",
            vec![worked("2026-08-25", 3_600), worked("2026-08-26", 1_800)],
        ),
    )
    .expect("a first publication should work");

    assert!(place.as_path().join("m-laptop/2026-08-25.json").is_file());
    assert!(place.as_path().join("m-laptop/2026-08-26.json").is_file());
    assert!(place.as_path().join("m-laptop").join(MANIFEST).is_file());
    // Two days and a manifest, and nothing else.
    assert_eq!(written.changed.len(), 3, "{:?}", written.changed);
}

#[test]
fn the_day_the_source_forgot_is_still_in_the_record() {
    // The failure this module exists to prevent. A collection made a month later
    // cannot see the old day at all, and must not be able to erase it.
    let place = root();
    publish(
        place.as_path(),
        &reading("2026-05-12T00:00:00Z", vec![worked("2026-05-12", 7_200)]),
    )
    .expect("the first publication should work");

    publish(
        place.as_path(),
        &reading("2026-08-26T00:00:00Z", vec![worked("2026-08-26", 1_800)]),
    )
    .expect("a later publication should work");

    let held = read_days(place.as_path(), "m-laptop").expect("the record should read back");
    let dates = held.iter().map(|day| day.date.as_str()).collect::<Vec<_>>();
    assert_eq!(dates, ["2026-05-12", "2026-08-26"]);
    assert_eq!(
        held[0].measure("agent").seconds,
        7_200,
        "the old day kept its hours"
    );
}

#[test]
fn a_thinner_reading_of_a_published_day_does_not_shrink_it() {
    // The window's edge: the day is still in the source, but only partly, so the
    // reading is short. The fuller reading already published is the true one.
    let place = root();
    publish(
        place.as_path(),
        &reading("2026-08-26T00:00:00Z", vec![worked("2026-08-01", 7_200)]),
    )
    .expect("the first publication should work");

    publish(
        place.as_path(),
        &reading("2026-08-27T00:00:00Z", vec![worked("2026-08-01", 600)]),
    )
    .expect("a later publication should work");

    let held = read_days(place.as_path(), "m-laptop").expect("the record should read back");
    assert_eq!(held[0].measure("agent").seconds, 7_200);
    assert_eq!(held[0].measure("agent").languages.get("Rust"), Some(&3_600));
}

#[test]
fn more_work_on_a_day_already_recorded_is_taken() {
    let place = root();
    publish(
        place.as_path(),
        &reading("2026-08-26T06:00:00Z", vec![worked("2026-08-26", 1_800)]),
    )
    .expect("the first publication should work");

    let written = publish(
        place.as_path(),
        &reading("2026-08-26T18:00:00Z", vec![worked("2026-08-26", 7_200)]),
    )
    .expect("a later publication should work");

    let held = read_days(place.as_path(), "m-laptop").expect("the record should read back");
    assert_eq!(held[0].measure("agent").seconds, 7_200);
    assert!(
        written
            .changed
            .iter()
            .any(|path| path.ends_with("2026-08-26.json")),
        "the day that changed should be reported: {:?}",
        written.changed
    );
}

#[test]
fn collecting_again_with_nothing_new_writes_nothing() {
    // What a daemon on a timer does almost every time it wakes. If this wrote,
    // every run would be a commit.
    let place = root();
    let days = vec![worked("2026-08-26", 3_600)];
    publish(
        place.as_path(),
        &reading("2026-08-26T06:00:00Z", days.clone()),
    )
    .expect("the first publication should work");

    // A later clock reading, the same work.
    let written = publish(place.as_path(), &reading("2026-08-26T06:30:00Z", days))
        .expect("a second publication should work");

    assert!(
        written.is_empty(),
        "nothing new should mean nothing written: {written:?}"
    );
}

#[test]
fn only_the_day_that_changed_is_rewritten() {
    // The reason for one file per day: a reader can fetch a window, and the
    // history says which day each commit recorded.
    let place = root();
    publish(
        place.as_path(),
        &reading(
            "2026-08-26T06:00:00Z",
            (1..=20)
                .map(|day| worked(&format!("2026-08-{day:02}"), 3_600))
                .collect(),
        ),
    )
    .expect("the first publication should work");

    let mut later = (1..=20)
        .map(|day| worked(&format!("2026-08-{day:02}"), 3_600))
        .collect::<Vec<_>>();
    later.push(worked("2026-08-21", 1_800));

    let written = publish(place.as_path(), &reading("2026-08-27T06:00:00Z", later))
        .expect("a later publication should work");

    let days = written
        .changed
        .iter()
        .filter(|path| !path.ends_with(MANIFEST))
        .collect::<Vec<_>>();
    assert_eq!(days.len(), 1, "twenty untouched days stayed untouched");
    assert!(days[0].ends_with("2026-08-21.json"), "{days:?}");
}

#[test]
fn the_manifest_totals_the_days_that_survived() {
    let place = root();
    publish(
        place.as_path(),
        &reading("2026-05-12T00:00:00Z", vec![worked("2026-05-12", 7_200)]),
    )
    .expect("the first publication should work");
    publish(
        place.as_path(),
        &reading("2026-08-26T00:00:00Z", vec![worked("2026-08-26", 1_800)]),
    )
    .expect("a later publication should work");

    let body = fs::read_to_string(place.as_path().join("m-laptop").join(MANIFEST))
        .expect("a manifest should be there");
    let manifest = parse_manifest(&body).expect("a manifest should read");

    assert_eq!(manifest.days, ["2026-05-12", "2026-08-26"]);
    let rollup = manifest.rollup.expect("two days should roll up");
    assert_eq!(rollup.first_day, "2026-05-12");
    assert_eq!(rollup.last_day, "2026-08-26");
    assert_eq!(
        rollup.measure("agent").seconds,
        9_000,
        "the lifetime total spans both"
    );
    assert_eq!(rollup.active_days, 2);
}

#[test]
fn two_machines_do_not_touch_each_others_days() {
    let place = root();
    publish(
        place.as_path(),
        &reading("2026-08-26T00:00:00Z", vec![worked("2026-08-26", 3_600)]),
    )
    .expect("the laptop should publish");

    let mut desktop = ActivitySnapshot::new("m-desktop", "2026-08-26T00:00:00Z");
    desktop.days = vec![worked("2026-08-26", 1_800)];
    publish(place.as_path(), &desktop).expect("the desktop should publish");

    let laptop = read_days(place.as_path(), "m-laptop").expect("the laptop record should read");
    let other = read_days(place.as_path(), "m-desktop").expect("the desktop record should read");
    assert_eq!(laptop[0].measure("agent").seconds, 3_600);
    assert_eq!(other[0].measure("agent").seconds, 1_800);
}

#[test]
fn the_single_file_record_is_taken_in_rather_than_left_behind() {
    // Upgrading from the layout that kept one file per machine. That file holds
    // the only copy of days the sources have since forgotten, so it is read
    // before it is removed.
    let place = root();
    let older = reading(
        "2026-08-24T00:00:00Z",
        vec![worked("2026-05-12", 7_200), worked("2026-08-24", 3_600)],
    );
    fs::write(
        place.as_path().join("m-laptop.json"),
        github_personal_stats_core::write_activity_snapshot(&older).expect("it should write"),
    )
    .expect("the older record should be placed");

    let written = publish(
        place.as_path(),
        &reading("2026-08-26T00:00:00Z", vec![worked("2026-08-26", 1_800)]),
    )
    .expect("publishing should take the older record in");

    let held = read_days(place.as_path(), "m-laptop").expect("the record should read back");
    let dates = held.iter().map(|day| day.date.as_str()).collect::<Vec<_>>();
    assert_eq!(dates, ["2026-05-12", "2026-08-24", "2026-08-26"]);
    assert_eq!(held[0].measure("agent").seconds, 7_200);

    assert!(
        !place.as_path().join("m-laptop.json").exists(),
        "the superseded file should be gone"
    );
    assert_eq!(written.removed, [Path::new("m-laptop.json")]);
}

#[test]
fn a_day_file_that_cannot_be_read_stops_the_run() {
    // Skipping it would let the next thin reading be written over a day whose
    // only copy is the file that failed to parse.
    let place = root();
    publish(
        place.as_path(),
        &reading("2026-08-26T00:00:00Z", vec![worked("2026-08-26", 3_600)]),
    )
    .expect("the first publication should work");

    fs::write(
        place.as_path().join("m-laptop/2026-08-26.json"),
        "{ not a day",
    )
    .expect("the file should be damaged");

    let error = publish(
        place.as_path(),
        &reading("2026-08-27T00:00:00Z", vec![worked("2026-08-27", 1_800)]),
    )
    .expect_err("a damaged day should stop the run");
    assert!(
        error.to_string().contains("2026-08-26"),
        "the error should name the file: {error}"
    );
}

#[test]
fn nothing_published_yet_is_not_a_failure() {
    let place = root();

    let held = read_days(place.as_path(), "m-laptop").expect("an empty root should read as empty");

    assert!(held.is_empty());
}
