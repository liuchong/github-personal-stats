//! The panel is the only place a person reads these numbers directly, so it is
//! worth checking that it says what was collected, keeps the two measures apart,
//! and does not hand a filename to the browser as markup.

use github_personal_stats_core::{
    ActivitySnapshot, Author, DayBucket, TimeBucket, summarise_activity,
};
use github_personal_stats_daemon::panel::{page, summary_json};

fn timing(seconds: u64, sessions: u32, languages: &[(&str, u64)]) -> TimeBucket {
    TimeBucket {
        seconds,
        sessions,
        languages: languages
            .iter()
            .map(|(name, seconds)| ((*name).to_owned(), *seconds))
            .collect(),
    }
}

fn day(date: &str, agent: TimeBucket, editor: TimeBucket) -> DayBucket {
    let mut day = DayBucket::new(date);
    *day.measure_mut("agent") = agent;
    *day.measure_mut("editor") = editor;
    day
}

fn snapshot(machine: &str, days: Vec<DayBucket>) -> ActivitySnapshot {
    ActivitySnapshot {
        schema: 1,
        machine: machine.to_owned(),
        collected_at: "2026-08-24T19:00:00Z".to_owned(),
        days,
        cursors: Default::default(),
    }
}

fn rendered(snapshot: &ActivitySnapshot) -> String {
    page(snapshot, &summarise_activity(&snapshot.days))
}

#[test]
fn the_page_names_the_machine_and_the_span_it_covers() {
    let snapshot = snapshot(
        "m-1234abcd",
        vec![
            day(
                "2026-08-22",
                timing(3_600, 2, &[("Rust", 3_600)]),
                timing(0, 0, &[]),
            ),
            day(
                "2026-08-24",
                timing(1_800, 1, &[("Rust", 1_800)]),
                timing(0, 0, &[]),
            ),
        ],
    );

    let html = rendered(&snapshot);

    assert!(html.starts_with("<!doctype html>"), "{}", &html[..40]);
    assert!(html.contains("Activity on m-1234abcd"));
    assert!(html.contains("2026-08-22 to 2026-08-24"));
    assert!(html.contains("2 days recorded"));
}

#[test]
fn an_empty_snapshot_renders_a_page_that_says_so_rather_than_breaking() {
    let html = rendered(&snapshot("m-new", Vec::new()));

    assert!(html.contains("nothing collected yet"), "{html}");
    assert!(html.contains("0 days recorded"));
    assert!(html.ends_with("</html>\n"));
}

#[test]
fn the_two_measures_are_reported_separately() {
    // The whole point of the split: a day can be long in one and short in the
    // other, and averaging them would hide that.
    let snapshot = snapshot(
        "m-1234abcd",
        vec![day(
            "2026-08-24",
            timing(7_200, 3, &[("Rust", 7_200)]),
            timing(600, 1, &[("Markdown", 600)]),
        )],
    );

    let html = rendered(&snapshot);

    assert!(html.contains("Languages, by agent time"), "{html}");
    assert!(html.contains("Languages, by editor time"), "{html}");
    assert!(html.contains("Rust"));
    assert!(html.contains("Markdown"));
}

#[test]
fn the_language_columns_share_one_scale_so_a_small_column_is_not_inflated() {
    // Scaling each column to its own largest entry would draw ten minutes of
    // editor time as long a bar as forty hours of agent time.
    let snapshot = snapshot(
        "m-1234abcd",
        vec![day(
            "2026-08-24",
            timing(144_000, 9, &[("Rust", 144_000)]),
            timing(600, 1, &[("Markdown", 600)]),
        )],
    );

    let html = rendered(&snapshot);
    let widths: Vec<f64> = html
        .split("width:")
        .skip(1)
        .filter_map(|rest| rest.split('%').next())
        .filter_map(|value| value.trim().parse().ok())
        .collect();

    let widest = widths.iter().cloned().fold(0.0_f64, f64::max);
    let narrowest = widths.iter().cloned().fold(100.0_f64, f64::min);
    assert!(
        widest > 90.0,
        "the largest entry should fill its bar: {widths:?}"
    );
    assert!(
        narrowest < 10.0,
        "ten minutes beside forty hours should be short: {widths:?}"
    );
}

#[test]
fn a_language_with_time_is_never_drawn_as_nothing_at_all() {
    let snapshot = snapshot(
        "m-1234abcd",
        vec![day(
            "2026-08-24",
            timing(360_060, 9, &[("Rust", 360_000), ("TOML", 60)]),
            timing(0, 0, &[]),
        )],
    );

    let html = rendered(&snapshot);
    let toml = html
        .split("TOML")
        .nth(1)
        .expect("a language with time recorded should appear");

    assert!(
        !toml.contains("width: 0%") && !toml.contains("width:0%"),
        "a minute of work should still be visible"
    );
}

#[test]
fn lines_are_split_into_what_was_committed_and_what_the_editor_generated() {
    let mut bucket = day("2026-08-24", timing(3_600, 1, &[]), timing(0, 0, &[]));
    bucket.commits.agent_added = 400;
    bucket.commits.tab_added = 50;
    bucket.commits.human_added = 100;
    bucket.commits.blank_added = 20;
    bucket.commits.unattributed_added = 30;
    bucket.add_lines("", Author::Agent, "claude-opus", 900, 0);
    bucket.add_lines("", Author::Human, "", 200, 0);

    let html = rendered(&snapshot("m-1234abcd", vec![bucket]));

    assert!(html.contains("Lines committed"), "{html}");
    assert!(html.contains("Lines generated in the editor"));
    assert!(html.contains("claude-opus"));
}

#[test]
fn a_machine_name_is_escaped_rather_than_handed_to_the_browser_as_markup() {
    // Machine names are minted locally, but a page that trusts its inputs is a
    // habit worth not having.
    let snapshot = snapshot("<script>alert(1)</script>", Vec::new());

    let html = rendered(&snapshot);

    assert!(
        !html.contains("<script>alert"),
        "the name should be escaped"
    );
    assert!(html.contains("&lt;script&gt;"), "{html}");
}

#[test]
fn a_language_name_is_escaped_too() {
    let snapshot = snapshot(
        "m-1234abcd",
        vec![day(
            "2026-08-24",
            timing(60, 1, &[("<b>C++</b>", 60)]),
            timing(0, 0, &[]),
        )],
    );

    let html = rendered(&snapshot);

    assert!(!html.contains("<b>C++</b>"), "the name should be escaped");
    assert!(html.contains("&lt;b&gt;"), "{html}");
}

#[test]
fn the_summary_is_valid_json_carrying_both_measures() {
    let snapshot = snapshot(
        "m-1234abcd",
        vec![day(
            "2026-08-24",
            timing(7_200, 3, &[("Rust", 7_200)]),
            timing(600, 1, &[("Markdown", 600)]),
        )],
    );

    let json = summary_json(&snapshot, &summarise_activity(&snapshot.days));

    assert!(
        json.starts_with('{') && json.trim_end().ends_with('}'),
        "{json}"
    );
    assert!(json.contains("\"machine\""), "{json}");
    assert!(json.contains("7200"), "{json}");
    assert!(json.contains("600"), "{json}");
    assert_eq!(
        json.matches('{').count(),
        json.matches('}').count(),
        "braces should balance: {json}"
    );
}

#[test]
fn the_summary_quotes_a_machine_name_that_contains_a_quote() {
    let json = summary_json(&snapshot("m-\"odd\"", Vec::new()), &summarise_activity(&[]));

    assert!(json.contains("\\\""), "a quote should be escaped: {json}");
}
