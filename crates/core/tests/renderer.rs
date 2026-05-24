use github_personal_stats_core::{
    AggregatedStats, CardData, CodingActivityEntry, GithubClient, GithubStatsConfig, LanguageShare,
    MockGithubClient, OutputKind, StreakMode, StreakSummary, aggregate_card_data,
    aggregate_coding_activity, render_card, render_readme_section,
};

const FIXTURE: &str = include_str!("fixtures/github_user_data.json");
const DASHBOARD_SNAPSHOT: &str = include_str!("snapshots/dashboard.svg");
const STATS_SNAPSHOT: &str = include_str!("snapshots/stats.svg");
const README_SNAPSHOT: &str = include_str!("snapshots/coding_activity.md");

fn fixture_data() -> github_personal_stats_core::GithubData {
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
fn renderer_outputs_streak_activity_and_status_cards() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(500, 220)
        .unwrap();
    let streak = CardData::Streak(StreakSummary {
        current: 3,
        longest: 10,
        total_active_days: 20,
        total_contributions: 1234,
        current_start: Some("2026-05-22".to_owned()),
        current_end: Some("2026-05-24".to_owned()),
        longest_start: Some("2026-04-01".to_owned()),
        longest_end: Some("2026-04-10".to_owned()),
        mode: StreakMode::Daily,
    });
    let activity = CardData::Activity(aggregate_coding_activity(
        vec![
            CodingActivityEntry {
                language: "Rust".to_owned(),
                seconds: 3660,
            },
            CodingActivityEntry {
                language: "TypeScript".to_owned(),
                seconds: 7200,
            },
        ],
        5,
        &[],
        false,
    ));
    let status = CardData::Status { state: "ready" };

    let streak_svg = render_card(&streak, &config);
    let activity_svg = render_card(&activity, &config);
    let status_svg = render_card(&status, &config);

    assert!(streak_svg.contains("Current Streak"));
    assert!(streak_svg.contains("May 22 2026 - May 24 2026"));
    assert!(streak_svg.contains("1,234"));
    assert!(activity_svg.contains("Coding Activity"));
    assert!(activity_svg.contains("TypeScript 2 hrs 0 mins"));
    assert!(status_svg.contains("Service health"));
    assert!(status_svg.contains(">ready<"));
}

#[test]
fn renderer_supports_theme_variants_and_fallback_colors() {
    let stats = AggregatedStats {
        total_stars: 1,
        total_commits: 2,
        total_pull_requests: 3,
        total_issues: 4,
        total_reviews: 5,
        contributed_to: 6,
        score: 7,
        rank: "C",
    };
    let languages = CardData::Languages(vec![
        LanguageShare {
            name: "UnknownOne".to_owned(),
            size: 5,
            percentage_basis_points: 5_000,
        },
        LanguageShare {
            name: "UnknownTwo".to_owned(),
            size: 5,
            percentage_basis_points: 5_000,
        },
    ]);
    let mut dark_config = GithubStatsConfig::new("octo").unwrap();
    dark_config.theme = "dark".to_owned();
    let mut transparent_config = GithubStatsConfig::new("octo").unwrap();
    transparent_config.theme = "transparent".to_owned();

    let dark_svg = render_card(&CardData::Stats(stats), &dark_config);
    let transparent_svg = render_card(&languages, &transparent_config);

    assert!(dark_svg.contains("#0d1117"));
    assert!(dark_svg.contains("#57606a"));
    assert!(transparent_svg.contains("transparent"));
    assert!(transparent_svg.contains("#6f42c1"));
    assert!(transparent_svg.contains("#0969da"));
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

#[test]
fn readme_section_escapes_title_and_handles_zero_total() {
    let summary = aggregate_coding_activity(
        vec![CodingActivityEntry {
            language: "Rust".to_owned(),
            seconds: 0,
        }],
        5,
        &[],
        true,
    );
    let markdown = render_readme_section(&summary, "Coding <Activity>");

    assert!(markdown.starts_with("### Coding &lt;Activity&gt;"));
    assert!(markdown.contains("Rust ░░░░░░░░░░ 0 hrs 0 mins"));
    assert!(markdown.contains("Total: 0 hrs 0 mins"));
}
