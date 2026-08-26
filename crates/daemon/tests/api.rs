use std::{
    fs,
    path::{Path, PathBuf},
};

use github_personal_stats_collect::{
    Settings, presence, pulse,
    sink::{FileSink, Sink},
};
use github_personal_stats_daemon::{
    Daemon,
    http::{Request, Response},
    token,
};

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("gps-daemon-test-{name}"));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a scratch directory should be creatable");
    directory
}

fn daemon(root: &Path) -> Daemon {
    let snapshot = root.join("record");
    let sink: Box<dyn Sink + Send + Sync> = Box::new(FileSink {
        path: snapshot.clone(),
    });
    Daemon::new(
        Settings {
            home: root.to_path_buf(),
            state_dir: root.join("state"),
            snapshot,
            idle_timeout_seconds: 300,
        },
        sink,
    )
    .expect("a daemon should start against a scratch directory")
}

fn request(method: &str, path: &str, bearer: Option<&str>, body: &str) -> Request {
    Request {
        method: method.to_owned(),
        path: path.to_owned(),
        bearer: bearer.map(str::to_owned),
        body: body.to_owned(),
    }
}

fn batch(day: &str, seconds: &[i64]) -> String {
    let pulses = seconds
        .iter()
        .map(|at| format!("{{\"at\":{at},\"day\":\"{day}\",\"ext\":\"rs\",\"write\":true}}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"editor\":\"vscode\",\"pulses\":[{pulses}]}}")
}

#[test]
fn health_answers_without_a_token_so_a_plugin_can_look_before_it_reads_one() {
    let root = scratch("health");
    let daemon = daemon(&root);

    let answer = daemon.answer(&request("GET", "/v1/health", None, ""));

    assert_eq!(answer.status, 200);
}

#[test]
fn everything_that_writes_or_reads_wants_the_token() {
    let root = scratch("guarded");
    let daemon = daemon(&root);

    for (method, path) in [
        ("POST", "/v1/pulses"),
        ("POST", "/v1/collect"),
        ("GET", "/v1/summary"),
        ("GET", "/"),
    ] {
        let refused = daemon.answer(&request(method, path, None, "{}"));
        assert_eq!(refused.status, 401, "{method} {path} should want the token");

        let wrong = daemon.answer(&request(method, path, Some("not-the-token"), "{}"));
        assert_eq!(wrong.status, 401, "{method} {path} should check the token");
    }
}

#[test]
fn a_token_offered_as_a_query_parameter_opens_the_panel_for_a_browser() {
    let root = scratch("query-token");
    let daemon = daemon(&root);
    let path = format!("/v1/summary?token={}", daemon.token());

    // The scratch home has no editor database, so the work fails rather than the
    // door. Anything but 401 proves the token was accepted.
    let answer = daemon.answer(&request("GET", &path, None, ""));

    assert_ne!(answer.status, 401);
}

#[test]
fn pulses_are_accepted_and_land_in_the_journal() {
    let root = scratch("accept");
    let daemon = daemon(&root);
    let body = batch("2026-08-24", &[1_787_000_000, 1_787_000_060]);

    let answer = daemon.answer(&request("POST", "/v1/pulses", Some(daemon.token()), &body));

    assert_eq!(answer.status, 200);
    assert!(answer.body.contains("\"accepted\":2"));
    let journal = pulse::journal_directory(&root.join("state")).join("2026-08-24.jsonl");
    assert_eq!(fs::read_to_string(journal).unwrap().lines().count(), 2);
}

#[test]
fn a_day_or_an_editor_that_could_name_a_file_elsewhere_is_refused() {
    let root = scratch("hostile");
    let daemon = daemon(&root);

    for body in [
        "{\"editor\":\"../../etc\",\"pulses\":[{\"at\":1,\"day\":\"2026-08-24\"}]}",
        "{\"editor\":\"vscode\",\"pulses\":[{\"at\":1787000000,\"day\":\"../../passwd\"}]}",
        "{\"editor\":\"vscode\",\"pulses\":[{\"at\":1787000000,\"day\":\"2026-08-24\",\"ext\":\"../x\"}]}",
        "{\"editor\":\"vscode\",\"pulses\":[]}",
        "{\"editor\":\"vscode\",\"pulses\":[{\"at\":0,\"day\":\"2026-08-24\"}]}",
    ] {
        let answer = daemon.answer(&request("POST", "/v1/pulses", Some(daemon.token()), body));
        assert_eq!(answer.status, 400, "{body} should be refused");
    }

    assert!(
        !pulse::journal_directory(&root.join("state")).exists()
            || fs::read_dir(pulse::journal_directory(&root.join("state")))
                .unwrap()
                .next()
                .is_none(),
        "nothing refused should have been written down"
    );
}

#[test]
fn a_body_that_is_not_a_pulse_batch_is_answered_rather_than_crashed_on() {
    let root = scratch("garbage");
    let daemon = daemon(&root);

    let answer = daemon.answer(&request(
        "POST",
        "/v1/pulses",
        Some(daemon.token()),
        "not json at all",
    ));

    assert_eq!(answer.status, 400);
}

#[test]
fn an_unknown_path_and_an_unknown_method_are_told_apart() {
    let root = scratch("routing");
    let daemon = daemon(&root);

    assert_eq!(
        daemon
            .answer(&request("GET", "/v1/nothing", Some(daemon.token()), ""))
            .status,
        404
    );
    assert_eq!(
        daemon
            .answer(&request("DELETE", "/v1/pulses", Some(daemon.token()), ""))
            .status,
        405
    );
}

#[test]
fn the_daemon_refuses_to_listen_anywhere_but_this_machine() {
    let root = scratch("loopback");
    let daemon = daemon(&root);

    for address in ["0.0.0.0:7391", "192.168.1.10:7391", "[::]:7391"] {
        let refused = daemon.listen(address);
        assert!(refused.is_err(), "{address} should be refused");
    }

    assert!(daemon.listen("127.0.0.1:0").is_ok());
}

#[test]
fn a_token_is_kept_between_runs_so_a_plugin_is_configured_once() {
    let root = scratch("stable-token");
    let first = daemon(&root).token().to_owned();
    let second = daemon(&root).token().to_owned();

    assert_eq!(first, second);
    assert_eq!(first.len(), 64);
}

#[test]
fn comparing_a_token_does_not_stop_early_on_the_first_wrong_byte() {
    assert!(token::matches("abcd", Some("abcd")));
    assert!(!token::matches("abcd", Some("abce")));
    assert!(!token::matches("abcd", Some("abc")));
    assert!(!token::matches("abcd", Some("abcde")));
    assert!(!token::matches("abcd", None));
}

#[test]
fn a_refusal_says_what_was_wrong_without_repeating_the_body_back() {
    let problem = Response::problem(400, "day \"../x\" must look like 2026-08-24");

    assert_eq!(problem.status, 400);
    assert!(problem.body.contains("must look like"));
    assert!(problem.body.starts_with("{\"error\":\""));
}

#[test]
fn a_plugin_that_says_hello_is_known_even_before_it_reports() {
    let root = scratch("hello");
    let daemon = daemon(&root);

    let answer = daemon.answer(&request(
        "POST",
        "/v1/hello",
        Some(daemon.token()),
        "{\"editor\":\"vscode\",\"version\":\"1.4.0\"}",
    ));

    assert_eq!(answer.status, 200);
    let announced = presence::read(&root.join("state"));
    assert_eq!(announced.len(), 1);
    assert_eq!(announced[0].editor, "vscode");
    assert_eq!(announced[0].version, "1.4.0");
}

#[test]
fn saying_hello_twice_leaves_one_record() {
    let root = scratch("hello-twice");
    let daemon = daemon(&root);
    let body = "{\"editor\":\"vscode\",\"version\":\"1.4.0\"}";

    daemon.answer(&request("POST", "/v1/hello", Some(daemon.token()), body));
    daemon.answer(&request("POST", "/v1/hello", Some(daemon.token()), body));

    assert_eq!(presence::read(&root.join("state")).len(), 1);
}

#[test]
fn an_announcement_never_becomes_time_worked() {
    let root = scratch("hello-is-not-work");
    let daemon = daemon(&root);

    daemon.answer(&request(
        "POST",
        "/v1/hello",
        Some(daemon.token()),
        "{\"editor\":\"vscode\",\"version\":\"1.4.0\"}",
    ));

    // Presence is kept apart from the journal, so nothing an announcement does
    // can be mistaken for a day's work.
    assert!(pulse::read(&root.join("state"), 300).unwrap().is_empty());
}

#[test]
fn a_hostile_editor_name_cannot_arrive_by_saying_hello() {
    let root = scratch("hello-hostile");
    let daemon = daemon(&root);

    let answer = daemon.answer(&request(
        "POST",
        "/v1/hello",
        Some(daemon.token()),
        "{\"editor\":\"../../etc/passwd\",\"version\":\"1\"}",
    ));

    assert_eq!(answer.status, 400);
    assert!(presence::read(&root.join("state")).is_empty());
}

#[test]
fn saying_hello_needs_the_token_like_everything_else() {
    let root = scratch("hello-unauthorised");
    let daemon = daemon(&root);

    let answer = daemon.answer(&request(
        "POST",
        "/v1/hello",
        None,
        "{\"editor\":\"vscode\"}",
    ));

    assert_eq!(answer.status, 401);
}
