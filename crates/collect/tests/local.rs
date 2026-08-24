//! The small local facts: what time it is, which machine this is, and which
//! editors have said hello. Each is load-bearing for a snapshot that has to
//! merge cleanly with snapshots from other machines.

use std::{fs, path::PathBuf};

use github_personal_stats_collect::{
    clock,
    machine::{identity, state_directory},
    presence,
};

fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("gps-local-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("a scratch directory");
    root
}

#[test]
fn a_timestamp_is_written_in_the_one_shape_everything_else_reads() {
    // 24 August 2026, 19:00:00 UTC.
    assert_eq!(clock::utc_timestamp(1_787_598_000), "2026-08-24T19:00:00Z");
}

#[test]
fn the_epoch_itself_is_not_a_special_case() {
    assert_eq!(clock::utc_timestamp(0), "1970-01-01T00:00:00Z");
}

#[test]
fn a_leap_year_does_not_shift_the_date() {
    // 29 February 2024 exists, and a naive day count would call this 1 March.
    assert_eq!(clock::utc_timestamp(1_709_164_800), "2024-02-29T00:00:00Z");
}

#[test]
fn the_last_second_of_a_year_stays_in_that_year() {
    assert_eq!(clock::utc_timestamp(1_767_225_599), "2025-12-31T23:59:59Z");
}

#[test]
fn the_clock_reads_a_time_in_the_range_this_code_could_be_running_in() {
    let now = clock::now();

    assert!(now > 1_700_000_000, "not before this was written: {now}");
    let stamp = clock::utc_timestamp(now);
    assert_eq!(stamp.len(), 20, "{stamp}");
    assert!(stamp.ends_with('Z'), "{stamp}");
}

#[test]
fn a_machine_keeps_the_same_name_once_it_has_one() {
    // Snapshots are filed per machine, so a name that changed between runs would
    // accumulate duplicate history rather than extend it.
    let root = scratch("identity");

    let first = identity(&root).expect("a machine should be able to name itself");
    let second = identity(&root).expect("the name should be found again");

    assert_eq!(first, second);
    assert!(first.starts_with("m-"), "{first}");
}

#[test]
fn two_machines_do_not_pick_the_same_name() {
    let one = identity(&scratch("one")).unwrap();
    let other = identity(&scratch("other")).unwrap();

    assert_ne!(one, other);
}

#[test]
fn a_name_carries_nothing_about_who_or_where_this_is() {
    // The whole point of minting one: a hostname would identify the machine, and
    // a snapshot may be published somewhere public.
    let name = identity(&scratch("anonymous")).unwrap();

    let allowed = |c: char| c.is_ascii_hexdigit();
    assert!(
        name.strip_prefix("m-")
            .is_some_and(|rest| rest.chars().all(allowed)),
        "a name should be nothing but hex: {name}"
    );
}

#[test]
fn state_is_kept_somewhere_belonging_to_this_tool() {
    let path = state_directory(&PathBuf::from("/home/someone"));

    assert!(
        path.to_string_lossy().contains("github-personal-stats"),
        "{path:?}"
    );
    assert!(path.is_absolute(), "{path:?}");
}

#[test]
fn an_editor_that_has_said_nothing_leaves_nothing_to_read() {
    let announced = presence::read(&scratch("silent"));

    assert!(announced.is_empty());
}

#[test]
fn an_editor_that_announces_itself_can_be_read_back() {
    let root = scratch("hello");

    presence::announce(&root, "vscode", "1.4.0").expect("a hello should be recorded");
    let announced = presence::read(&root);

    assert_eq!(announced.len(), 1);
    assert_eq!(announced[0].editor, "vscode");
    assert_eq!(announced[0].version, "1.4.0");
    assert!(announced[0].at > 1_700_000_000);
}

#[test]
fn announcing_again_replaces_the_earlier_one_rather_than_piling_up() {
    // A window reloaded twenty times is still one editor.
    let root = scratch("reload");

    presence::announce(&root, "vscode", "1.4.0").unwrap();
    presence::announce(&root, "vscode", "1.4.1").unwrap();
    let announced = presence::read(&root);

    assert_eq!(announced.len(), 1);
    assert_eq!(announced[0].version, "1.4.1");
}

#[test]
fn two_editors_are_both_remembered() {
    let root = scratch("two-editors");

    presence::announce(&root, "vscode", "1.4.0").unwrap();
    presence::announce(&root, "neovim", "0.1.0").unwrap();

    assert_eq!(presence::read(&root).len(), 2);
}

#[test]
fn a_damaged_record_reads_as_nobody_rather_than_bringing_the_report_down() {
    // The file is one document, so a damaged one cannot be partly believed. It
    // says nobody announced, which is what a missing file would say too, and the
    // next editor to start will write it afresh.
    let root = scratch("damaged");
    presence::announce(&root, "vscode", "1.4.0").unwrap();
    fs::write(presence::path(&root), "{ not json at all").unwrap();

    assert!(presence::read(&root).is_empty());

    presence::announce(&root, "vscode", "1.4.0").expect("a later hello should still land");
    assert_eq!(presence::read(&root).len(), 1);
}

#[test]
fn the_most_recent_announcement_is_read_first() {
    let root = scratch("ordering");
    let path = presence::path(&root);
    fs::write(
        &path,
        r#"[{"editor":"old","version":"1","at":1000},
            {"editor":"new","version":"1","at":2000}]"#,
    )
    .unwrap();

    let announced = presence::read(&root);

    assert_eq!(announced[0].editor, "new");
    assert_eq!(announced[1].editor, "old");
}
