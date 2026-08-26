use github_personal_stats_collect::{presence::Announcement, pulse::Reporter};
use github_personal_stats_daemon::status::{self, Collected, Reading};

const NOW: i64 = 1_787_000_000;

fn announcement(editor: &str, version: &str, at: i64) -> Announcement {
    Announcement {
        editor: editor.to_owned(),
        version: version.to_owned(),
        at,
    }
}

fn reporter(editor: &str, pulses: usize, last_seen: i64) -> Reporter {
    Reporter {
        editor: editor.to_owned(),
        day: "2026-08-24".to_owned(),
        pulses,
        last_seen,
    }
}

fn reading<'a>(
    announced: &'a [Announcement],
    reporters: &'a [Reporter],
    collected: Option<Collected>,
) -> Reading<'a> {
    Reading {
        address: "127.0.0.1:7391",
        listening: true,
        token: Some("/state/token"),
        publishing: "file /state/record",
        announced,
        reporters,
        at: NOW,
        collected,
    }
}

#[test]
fn nothing_loaded_says_so_and_says_what_to_do() {
    let report = status::report(&reading(&[], &[], None));

    assert!(report.contains("no plugin has loaded"), "{report}");
    assert!(report.contains("reload the editor window"), "{report}");
}

#[test]
fn a_loaded_plugin_with_nothing_to_report_is_not_shown_as_a_fault() {
    // The ordinary state of a window an agent is working in. Counting only
    // pulses would make this indistinguishable from a plugin that never loaded.
    let announced = [announcement("vscode", "1.4.0", NOW - 720)];

    let report = status::report(&reading(&announced, &[], None));

    assert!(report.contains("vscode 1.4.0"), "{report}");
    assert!(report.contains("loaded 12m ago"), "{report}");
    assert!(report.contains("nothing reported recently"), "{report}");
    assert!(!report.contains("no plugin has loaded"), "{report}");
}

#[test]
fn a_plugin_that_is_reporting_shows_its_work_instead() {
    let announced = [announcement("vscode", "1.4.0", NOW - 7_200)];
    let reporters = [reporter("vscode", 412, NOW - 30)];

    let report = status::report(&reading(&announced, &reporters, None));

    assert!(report.contains("412 pulses on 2026-08-24"), "{report}");
    assert!(report.contains("last 30s ago"), "{report}");
    assert!(!report.contains("nothing typed"), "{report}");
}

#[test]
fn an_editor_heard_from_without_an_announcement_still_counts() {
    // An older plugin, or one whose hello was lost. Its work is not discarded.
    let reporters = [reporter("neovim", 12, NOW - 60)];

    let report = status::report(&reading(&[], &reporters, None));

    assert!(
        report.contains("neovim — 12 pulses on 2026-08-24"),
        "{report}"
    );
    assert!(!report.contains("no plugin has loaded"), "{report}");
}

#[test]
fn several_editors_each_get_a_line() {
    let announced = [
        announcement("vscode", "1.4.0", NOW - 60),
        announcement("neovim", "0.1.0", NOW - 60),
    ];
    let reporters = [reporter("vscode", 5, NOW - 10)];

    let report = status::report(&reading(&announced, &reporters, None));

    assert_eq!(
        report
            .lines()
            .filter(|line| line.contains("editors"))
            .count(),
        2,
        "{report}"
    );
}

#[test]
fn a_missing_token_is_called_out_because_nothing_can_report_without_it() {
    let mut reading = reading(&[], &[], None);
    reading.token = None;

    let report = status::report(&reading);

    assert!(report.contains("token       missing"), "{report}");
    assert!(report.contains("no plugin can report"), "{report}");
}

#[test]
fn a_daemon_that_is_not_up_is_distinguished_from_one_that_is() {
    let mut reading = reading(&[], &[], None);
    reading.listening = false;

    let report = status::report(&reading);

    assert!(
        report.contains("not listening on 127.0.0.1:7391"),
        "{report}"
    );
}

#[test]
fn what_has_been_collected_is_reported_as_two_separate_numbers() {
    let collected = Collected {
        days: 100,
        agent_seconds: 527_340,
        editor_seconds: 11_520,
    };

    let report = status::report(&reading(&[], &[], Some(collected)));

    assert!(report.contains("100 days"), "{report}");
    assert!(report.contains("agent 146h 29m"), "{report}");
    assert!(report.contains("editor 3h 12m"), "{report}");
}

#[test]
fn one_pulse_is_not_called_pulses() {
    let reporters = [reporter("emacs", 1, NOW - 30)];

    let report = status::report(&reading(&[], &reporters, None));

    assert!(report.contains("1 pulse on"), "{report}");
}

#[test]
fn a_plugin_without_a_version_is_still_named() {
    let announced = [announcement("vscode", "", NOW - 60)];

    let report = status::report(&reading(&announced, &[], None));

    assert!(report.contains("editors     vscode —"), "{report}");
}

#[test]
fn a_gap_is_said_in_whatever_unit_still_means_something() {
    assert_eq!(status::ago(0), "0s");
    assert_eq!(status::ago(45), "45s");
    assert_eq!(status::ago(90), "90s");
    assert_eq!(status::ago(91), "1m");
    assert_eq!(status::ago(720), "12m");
    assert_eq!(status::ago(5_400), "90m");
    assert_eq!(status::ago(5_401), "1h");
    assert_eq!(status::ago(7_200), "2h");
}

#[test]
fn a_clock_running_ahead_reads_as_just_now_rather_than_backwards() {
    // Two machines rarely agree to the second, and a negative duration would be
    // read as a fault rather than as a clock.
    assert_eq!(status::ago(-30), "0s");
}

#[test]
fn a_duration_is_shown_as_hours_and_minutes() {
    assert_eq!(status::clock_face(0), "0h 0m");
    assert_eq!(status::clock_face(59), "0h 0m");
    assert_eq!(status::clock_face(3_600), "1h 0m");
    assert_eq!(status::clock_face(527_340), "146h 29m");
}
