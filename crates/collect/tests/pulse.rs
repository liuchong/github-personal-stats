use std::{fs, path::PathBuf};

use github_personal_stats_collect::pulse::{self, Pulse, PulseBatch};

const DAY: &str = "2026-08-24";
const NOON: i64 = 1_787_000_000;

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("gps-pulse-test-{name}"));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a scratch directory should be creatable");
    directory
}

fn batch(pulses: Vec<(i64, &str)>) -> PulseBatch {
    PulseBatch {
        editor: "vscode".to_owned(),
        pulses: pulses
            .into_iter()
            .map(|(offset, ext)| Pulse {
                at: NOON + offset,
                day: DAY.to_owned(),
                ext: ext.to_owned(),
                write: true,
            })
            .collect(),
    }
}

#[test]
fn a_gap_within_the_timeout_is_time_worked() {
    let root = scratch("gap");
    pulse::append(&root, &batch(vec![(0, "rs"), (60, "rs"), (120, "rs")])).unwrap();

    let days = pulse::read(&root, 300).unwrap();

    assert_eq!(days[DAY].seconds, 120);
    assert_eq!(days[DAY].sessions, 1);
    assert_eq!(days[DAY].languages["Rust"], 120);
}

#[test]
fn a_gap_past_the_timeout_ends_the_session_instead_of_counting_as_work() {
    let root = scratch("break");
    pulse::append(
        &root,
        &batch(vec![(0, "rs"), (60, "rs"), (7200, "rs"), (7260, "rs")]),
    )
    .unwrap();

    let days = pulse::read(&root, 300).unwrap();

    assert_eq!(days[DAY].seconds, 120, "the two hour break is not work");
    assert_eq!(days[DAY].sessions, 2);
}

#[test]
fn a_plugin_that_sends_the_same_batch_twice_does_not_inflate_the_record() {
    let root = scratch("resend");
    let sent = batch(vec![(0, "rs"), (60, "rs"), (120, "rs")]);

    pulse::append(&root, &sent).unwrap();
    let once = pulse::read(&root, 300).unwrap();
    pulse::append(&root, &sent).unwrap();
    let twice = pulse::read(&root, 300).unwrap();

    assert_eq!(once[DAY].seconds, twice[DAY].seconds);
    assert_eq!(once[DAY].sessions, twice[DAY].sessions);
}

#[test]
fn reading_the_journal_again_gives_the_same_answer() {
    let root = scratch("idempotent");
    pulse::append(&root, &batch(vec![(0, "rs"), (90, "md")])).unwrap();

    assert_eq!(
        pulse::read(&root, 300).unwrap(),
        pulse::read(&root, 300).unwrap()
    );
}

#[test]
fn time_follows_the_file_that_was_open_when_the_gap_started() {
    let root = scratch("languages");
    pulse::append(&root, &batch(vec![(0, "rs"), (60, "md"), (120, "md")])).unwrap();

    let days = pulse::read(&root, 300).unwrap();

    assert_eq!(days[DAY].languages["Rust"], 60);
    assert_eq!(days[DAY].languages["Markdown"], 60);
}

#[test]
fn a_file_kind_nobody_recognises_is_counted_rather_than_dropped() {
    let root = scratch("unknown");
    pulse::append(&root, &batch(vec![(0, "wat"), (60, "wat")])).unwrap();

    let days = pulse::read(&root, 300).unwrap();

    assert_eq!(days[DAY].languages["Other"], 60);
}

#[test]
fn an_empty_extension_is_allowed_because_not_every_file_has_one() {
    let root = scratch("no-extension");

    assert!(pulse::append(&root, &batch(vec![(0, ""), (60, "")])).is_ok());
}

#[test]
fn nothing_is_written_down_before_the_batch_is_checked() {
    let root = scratch("refused");
    let mut hostile = batch(vec![(0, "rs")]);
    hostile.editor = "../../etc".to_owned();

    assert!(pulse::append(&root, &hostile).is_err());
    assert!(!pulse::journal_directory(&root).exists());
}

#[test]
fn no_journal_yet_is_no_time_rather_than_an_error() {
    let root = scratch("missing");

    assert!(pulse::read(&root, 300).unwrap().is_empty());
}

#[test]
fn each_day_is_its_own_file_so_two_days_never_collide() {
    let root = scratch("days");
    let mut spanning = batch(vec![(0, "rs"), (60, "rs")]);
    spanning.pulses.push(Pulse {
        at: NOON + 86_400,
        day: "2026-08-25".to_owned(),
        ext: "rs".to_owned(),
        write: true,
    });

    pulse::append(&root, &spanning).unwrap();

    let directory = pulse::journal_directory(&root);
    assert!(directory.join("2026-08-24.jsonl").exists());
    assert!(directory.join("2026-08-25.jsonl").exists());
}

#[test]
fn the_journal_keeps_no_paths_and_no_project_names() {
    let root = scratch("privacy");
    pulse::append(&root, &batch(vec![(0, "rs"), (60, "rs")])).unwrap();

    let written =
        fs::read_to_string(pulse::journal_directory(&root).join("2026-08-24.jsonl")).unwrap();

    assert!(written.contains("\"ext\":\"rs\""));
    assert!(
        !written.contains('/'),
        "a path could have got in: {written}"
    );
}
