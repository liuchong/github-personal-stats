use github_stats_core::{
    CodingActivityEntry, GithubClient, GithubStatsConfig, MockGithubClient, OutputKind,
    aggregate_card_data, aggregate_coding_activity, render_card, render_readme_section,
};

const FIXTURE: &str = include_str!("fixtures/github_user_data.json");
const DASHBOARD_SNAPSHOT: &str = include_str!("snapshots/dashboard.svg");
const STATS_SNAPSHOT: &str = include_str!("snapshots/stats.svg");
const README_SNAPSHOT: &str = include_str!("snapshots/coding_activity.md");

fn fixture_data() -> github_stats_core::GithubData {
    let config = GithubStatsConfig::new("octo").unwrap();
    MockGithubClient::success(FIXTURE)
        .fetch_user_data(&config)
        .unwrap()
}

#[test]
fn dashboard_renderer_matches_snapshot() {
    let config = GithubStatsConfig::new("octo").unwrap();
    let card = aggregate_card_data(&fixture_data(), OutputKind::Dashboard);
    let svg = render_card(&card, &config);

    assert_eq!(svg, DASHBOARD_SNAPSHOT.trim_end());
}

#[test]
fn stats_renderer_matches_snapshot() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(420, 220)
        .unwrap();
    let card = aggregate_card_data(&fixture_data(), OutputKind::Stats);
    let svg = render_card(&card, &config);

    assert_eq!(svg, STATS_SNAPSHOT.trim_end());
}

#[test]
fn renderer_sets_fixed_svg_dimensions() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(700, 300)
        .unwrap();
    let card = aggregate_card_data(&fixture_data(), OutputKind::Languages);
    let svg = render_card(&card, &config);

    assert!(svg.contains(r#"width="700""#));
    assert!(svg.contains(r#"height="300""#));
    assert!(svg.contains(r#"viewBox="0 0 700 300""#));
}

#[test]
fn language_renderer_uses_language_specific_colors() {
    let config = GithubStatsConfig::new("octo").unwrap();
    let card = aggregate_card_data(&fixture_data(), OutputKind::Languages);
    let svg = render_card(&card, &config);

    assert!(svg.contains("#dea584"));
    assert!(svg.contains("#3178c6"));
    assert!(svg.contains("#89e051"));
}

#[test]
fn readme_section_renderer_matches_snapshot() {
    let summary = aggregate_coding_activity(
        vec![
            CodingActivityEntry {
                language: "Rust".to_owned(),
                seconds: 7200,
            },
            CodingActivityEntry {
                language: "Shell".to_owned(),
                seconds: 1800,
            },
        ],
        5,
        &[],
        false,
    );
    let markdown = render_readme_section(&summary, "Coding Activity");

    assert_eq!(markdown, README_SNAPSHOT.trim_end());
}
