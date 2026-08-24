//! How a day, and a machine's index of days, survive a trip through a file.
//!
//! These files are the boundary between a laptop that collects and a workflow
//! that renders, and the two can be running different builds, so the checks that
//! matter here are the ones about a file being readable on its own.

use github_personal_stats_core::{
    ACTIVITY_SCHEMA, DayBucket, LineCounts,
    store::{
        DayRecord, MachineManifest, day_file, machine_directory, manifest_file, parse_day,
        parse_manifest, roll_up, write_day, write_manifest,
    },
};

fn worked(date: &str, editor: u64, agent: u64) -> DayBucket {
    let mut day = DayBucket::new(date);
    day.editor.seconds = editor;
    day.editor.sessions = 1;
    day.editor.languages.insert("Rust".to_owned(), editor);
    day.agent.seconds = agent;
    day.agent.sessions = 1;
    day.requests = 3;
    day.committed = LineCounts {
        agent_added: 10,
        human_added: 5,
        ..LineCounts::default()
    };
    day
}

#[test]
fn a_day_is_readable_on_its_own() {
    let record = DayRecord::new("m-laptop", worked("2026-08-24", 3_600, 1_800));

    let written = write_day(&record).expect("a day should write");
    let read_back = parse_day(&written).expect("a day should read");

    assert_eq!(read_back, record);
    // The machine is in the file, not only in the path, so a day that has been
    // fetched over HTTP or copied by hand still says whose it is.
    assert!(written.contains("m-laptop"), "{written}");
    assert!(written.contains("2026-08-24"), "{written}");
}

#[test]
fn a_day_from_a_newer_build_is_declined_rather_than_half_read() {
    let mut record = DayRecord::new("m-laptop", worked("2026-08-24", 60, 60));
    record.schema = ACTIVITY_SCHEMA + 1;

    let written = serde_json::to_string(&record).unwrap();
    let error = parse_day(&written).expect_err("a newer schema should be refused");

    assert!(error.to_string().contains("newer"), "{error}");
}

#[test]
fn a_day_that_could_not_name_a_file_is_refused() {
    let mut record = DayRecord::new("m-laptop", worked("2026-08-24", 60, 60));
    record.day.date = "24 August".to_owned();
    assert!(write_day(&record).is_err());

    let mut sideways = DayRecord::new("../../etc", worked("2026-08-24", 60, 60));
    sideways.day.date = "2026-08-24".to_owned();
    assert!(
        write_day(&sideways).is_err(),
        "a machine id has to be safe to use as a directory name"
    );
}

#[test]
fn nothing_collected_rolls_up_to_nothing() {
    // Not a zeroed rollup: a first and last day would have to be invented.
    assert!(roll_up(&[]).is_none());
}

#[test]
fn a_rollup_spans_the_days_and_sums_them() {
    let days = vec![
        worked("2026-08-24", 3_600, 1_800),
        worked("2026-08-20", 1_800, 900),
        DayBucket::new("2026-08-22"),
    ];

    let rollup = roll_up(&days).expect("days should roll up");

    assert_eq!(rollup.first_day, "2026-08-20");
    assert_eq!(rollup.last_day, "2026-08-24");
    // The quiet day sits inside the span but is not active work.
    assert_eq!(rollup.active_days, 2);
    assert_eq!(rollup.editor.seconds, 5_400);
    assert_eq!(rollup.agent.seconds, 2_700);
    assert_eq!(rollup.editor.languages.get("Rust"), Some(&5_400));
    assert_eq!(rollup.requests, 6);
    assert_eq!(rollup.committed.agent_added, 20);
    assert_eq!(rollup.committed.human_added, 10);
}

#[test]
fn a_rollup_reads_the_same_whichever_order_the_days_arrive() {
    let forwards = roll_up(&[worked("2026-08-20", 60, 30), worked("2026-08-24", 60, 30)]);
    let backwards = roll_up(&[worked("2026-08-24", 60, 30), worked("2026-08-20", 60, 30)]);

    assert_eq!(forwards, backwards);
}

#[test]
fn a_manifest_indexes_its_days_and_carries_their_sum() {
    let days = vec![
        worked("2026-08-24", 3_600, 1_800),
        worked("2026-08-20", 1_800, 900),
    ];

    let manifest = MachineManifest::new("m-laptop", "2026-08-24T19:00:00Z").describing(&days);

    assert_eq!(manifest.days, vec!["2026-08-20", "2026-08-24"]);
    let rollup = manifest.rollup.as_ref().expect("a rollup");
    assert_eq!(rollup.editor.seconds, 5_400);
}

#[test]
fn a_manifest_survives_a_trip_through_a_file() {
    let days = vec![worked("2026-08-24", 3_600, 1_800)];
    let mut manifest = MachineManifest::new("m-laptop", "2026-08-24T19:00:00Z").describing(&days);
    manifest
        .cursors
        .insert("transcripts".to_owned(), "2026-08-24T18:00:00Z".to_owned());

    let written = write_manifest(&manifest).expect("a manifest should write");
    let read_back = parse_manifest(&written).expect("a manifest should read");

    assert_eq!(read_back, manifest);
}

#[test]
fn collecting_again_without_working_is_not_a_change() {
    let days = vec![worked("2026-08-24", 3_600, 1_800)];
    let earlier = MachineManifest::new("m-laptop", "2026-08-24T19:00:00Z").describing(&days);
    let later = MachineManifest::new("m-laptop", "2026-08-24T20:30:00Z").describing(&days);

    assert!(
        earlier.records_the_same_as(&later),
        "only the clock moved, so this must not read as new work"
    );

    let more = MachineManifest::new("m-laptop", "2026-08-24T20:30:00Z").describing(&[worked(
        "2026-08-24",
        7_200,
        1_800,
    )]);
    assert!(!earlier.records_the_same_as(&more));
}

#[test]
fn two_machines_never_write_to_the_same_path() {
    let root = std::path::Path::new("/snapshots");

    let laptop = day_file(root, "m-laptop", "2026-08-24");
    let desktop = day_file(root, "m-desktop", "2026-08-24");

    assert_ne!(laptop, desktop);
    assert!(laptop.ends_with("m-laptop/2026-08-24.json"), "{laptop:?}");
    assert_eq!(
        manifest_file(root, "m-laptop").parent(),
        Some(machine_directory(root, "m-laptop").as_path())
    );
}

#[test]
fn a_day_is_one_file_so_a_later_day_leaves_it_alone() {
    let root = std::path::Path::new("/snapshots");

    assert_ne!(
        day_file(root, "m-laptop", "2026-08-24"),
        day_file(root, "m-laptop", "2026-08-25"),
        "recording today must not rewrite yesterday"
    );
}
