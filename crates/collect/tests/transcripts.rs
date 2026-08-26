//! Tokens read out of the records terminal agents leave behind.
//!
//! The two agents report differently — one keeps a running total and the cost of
//! the last request, the other reports each message as it arrives — and both have
//! a notion of a cache whose reads were not paid for at the full rate. These
//! tests are mostly about getting those two things right, and about the day a
//! request lands on when a session runs past midnight.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};

use github_personal_stats_collect::transcripts;
use github_personal_stats_core::DayBucket;

fn scratch() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let directory = std::env::temp_dir().join(format!(
        "gps-transcripts-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a scratch directory should be creatable");
    directory
}

fn write(path: &Path, lines: &[&str]) {
    fs::create_dir_all(path.parent().expect("a file has a parent"))
        .expect("a directory should be creatable");
    fs::write(path, format!("{}\n", lines.join("\n"))).expect("a transcript should be writable");
}

fn codex_event(stamp: &str, input: u64, cached: u64, output: u64) -> String {
    format!(
        r#"{{"timestamp":"{stamp}","type":"event_msg","payload":{{"type":"token_count","info":{{"last_token_usage":{{"input_tokens":{input},"cached_input_tokens":{cached},"output_tokens":{output}}},"total_token_usage":{{"input_tokens":0,"cached_input_tokens":0,"output_tokens":0}}}}}}}}"#
    )
}

fn read(home: &Path) -> BTreeMap<String, DayBucket> {
    let mut days = BTreeMap::new();
    transcripts::read(home, &mut days).expect("a readable record should be read");
    days
}

fn spent(days: &BTreeMap<String, DayBucket>, date: &str, model: &str) -> (u64, u64, u64) {
    days.get(date)
        .and_then(|day| day.tokens.get(model))
        .map(|usage| (usage.input, usage.output, usage.cached))
        .unwrap_or_default()
}

#[test]
fn codex_tokens_are_counted_per_request_and_kept_apart_from_the_cache() {
    let home = scratch();
    write(
        &transcripts::codex_path(&home).join("2026/08/20/rollout-one.jsonl"),
        &[
            r#"{"timestamp":"2026-08-20T04:00:00.000Z","type":"turn_context","payload":{"model":"gpt-5.6"}}"#,
            &codex_event("2026-08-20T04:00:01.000Z", 1_000, 800, 50),
            &codex_event("2026-08-20T04:10:00.000Z", 2_000, 1_900, 70),
        ],
    );

    let days = read(&home);
    let day = days.keys().next().expect("one day should be recorded");
    // Codex counts cached input inside its input figure, so the record's input is
    // what the cache did not serve: three thousand asked for, two thousand seven
    // hundred served from cache.
    assert_eq!(spent(&days, day, "gpt-5.6"), (300, 120, 2_700));
}

#[test]
fn claude_tokens_are_summed_as_they_arrive_and_a_cache_write_is_paid_for() {
    let home = scratch();
    write(
        &transcripts::claude_path(&home).join("a-project/session.jsonl"),
        &[
            r#"{"timestamp":"2026-08-20T04:00:00.000Z","message":{"model":"claude-opus-5","usage":{"input_tokens":100,"output_tokens":10,"cache_read_input_tokens":900,"cache_creation_input_tokens":40}}}"#,
            r#"{"timestamp":"2026-08-20T04:05:00.000Z","message":{"model":"claude-opus-5","usage":{"input_tokens":200,"output_tokens":20,"cache_read_input_tokens":1000,"cache_creation_input_tokens":0}}}"#,
        ],
    );

    let days = read(&home);
    let day = days.keys().next().expect("one day should be recorded");
    // Reading a cache is free and writing one is not, so the write counts as
    // input: three hundred asked for plus forty written.
    assert_eq!(spent(&days, day, "claude-opus-5"), (340, 30, 1_900));
}

#[test]
fn a_session_that_runs_into_another_day_spends_on_both() {
    let home = scratch();
    // More than a day apart, so the two requests land on different local days
    // whatever the machine's offset from UTC happens to be.
    write(
        &transcripts::codex_path(&home).join("2026/08/20/rollout-long.jsonl"),
        &[
            r#"{"timestamp":"2026-08-20T00:00:00.000Z","type":"turn_context","payload":{"model":"gpt-5.6"}}"#,
            &codex_event("2026-08-20T00:00:00.000Z", 100, 0, 10),
            &codex_event("2026-08-21T01:00:00.000Z", 200, 0, 20),
        ],
    );

    let days = read(&home);
    assert_eq!(
        days.len(),
        2,
        "a session spanning two days should spend on both, got {:?}",
        days.keys().collect::<Vec<_>>()
    );
    let counted = days
        .values()
        .filter_map(|day| day.tokens.get("gpt-5.6"))
        .map(|usage| usage.input + usage.output)
        .sum::<u64>();
    assert_eq!(counted, 330);
}

#[test]
fn a_home_with_neither_agent_installed_is_not_an_error() {
    // A machine that only ever ran one of these, or neither, should not have to
    // say so for a collection to succeed.
    let days = read(&scratch());
    assert!(days.is_empty());
}

#[test]
fn a_request_that_cost_nothing_leaves_no_entry() {
    let home = scratch();
    write(
        &transcripts::codex_path(&home).join("2026/08/20/rollout-empty.jsonl"),
        &[
            r#"{"timestamp":"2026-08-20T04:00:00.000Z","type":"turn_context","payload":{"model":"gpt-5.6"}}"#,
            &codex_event("2026-08-20T04:00:01.000Z", 0, 0, 0),
        ],
    );

    // An aborted turn reports zeroes. Recording a model that spent nothing would
    // put a row on a chart with nothing in it.
    assert!(read(&home).is_empty());
}

#[test]
fn reading_the_same_record_twice_counts_it_once() {
    let home = scratch();
    write(
        &transcripts::codex_path(&home).join("2026/08/20/rollout-one.jsonl"),
        &[
            r#"{"timestamp":"2026-08-20T04:00:00.000Z","type":"turn_context","payload":{"model":"gpt-5.6"}}"#,
            &codex_event("2026-08-20T04:00:01.000Z", 1_000, 0, 100),
        ],
    );

    // A collection runs on a timer over files that mostly have not changed, so
    // two readings of one file must not read as twice the work.
    let once = read(&home);
    let twice = read(&home);
    assert_eq!(once, twice);
}

#[test]
fn a_model_the_agent_did_not_name_is_still_counted() {
    let home = scratch();
    write(
        &transcripts::claude_path(&home).join("a-project/session.jsonl"),
        &[
            r#"{"timestamp":"2026-08-20T04:00:00.000Z","message":{"usage":{"input_tokens":100,"output_tokens":10}}}"#,
        ],
    );

    let days = read(&home);
    let day = days.keys().next().expect("one day should be recorded");
    assert_eq!(spent(&days, day, "unnamed"), (100, 10, 0));
}
