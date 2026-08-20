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
fn renderer_outputs_streak_wakatime_and_status_cards() {
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
        recent_daily_counts: vec![0, 1, 2, 3, 4],
    });
    let wakatime = CardData::Wakatime(aggregate_coding_activity(
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
    let wakatime_svg = render_card(&wakatime, &config);
    let status_svg = render_card(&status, &config);

    assert!(streak_svg.contains("Current Streak"));
    assert!(streak_svg.contains("May 22 2026 – May 24 2026"));
    assert!(streak_svg.contains("1,234"));
    assert!(wakatime_svg.contains("CODING ACTIVITY"));
    assert!(wakatime_svg.contains(">TypeScript<"));
    assert!(wakatime_svg.contains(">2 hrs 0 mins<"));
    assert!(status_svg.contains("Service health"));
    assert!(status_svg.contains(">ready<"));
}

#[test]
fn rank_ring_closure_follows_ranking_percentile() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(480, 200)
        .unwrap();
    let top = CardData::Stats(sample_stats(100));
    let bottom = CardData::Stats(sample_stats(9_000));

    let top_svg = render_card(&top, &config);
    let bottom_svg = render_card(&bottom, &config);

    let closed = dash_array(&top_svg);
    let open = dash_array(&bottom_svg);

    assert!(
        closed > open,
        "a better percentile must close more of the ring: {closed} vs {open}"
    );
}

#[test]
fn streak_heat_ring_draws_one_tick_per_recent_day_along_the_fire_ramp() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(500, 220)
        .unwrap();
    let counts = vec![0, 1, 2, 3, 4, 5, 0, 2];

    let svg = render_card(&sample_streak(counts.clone()), &config);

    assert_eq!(svg.matches(r#"stroke-width="3.6""#).count(), counts.len());
    assert!(svg.contains("#fb8c00"), "busiest day uses the hottest tone");
    assert!(
        svg.contains("#ffe3ad"),
        "quietest active day uses the palest tone"
    );
    assert!(
        svg.contains("#e4e8ee"),
        "days without activity stay neutral"
    );
}

#[test]
fn streak_heat_ring_falls_back_to_a_plain_ring_without_recent_data() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(500, 220)
        .unwrap();
    let streak = CardData::Streak(StreakSummary {
        current: 0,
        longest: 0,
        total_active_days: 0,
        total_contributions: 0,
        current_start: None,
        current_end: None,
        longest_start: None,
        longest_end: None,
        mode: StreakMode::Daily,
        recent_daily_counts: Vec::new(),
    });

    let svg = render_card(&streak, &config);

    assert!(!svg.contains(r#"stroke-width="3.6""#));
    assert!(svg.contains(r##"stroke="#e4e8ee" stroke-width="3""##));
}

#[test]
fn language_rows_switch_layout_with_available_width() {
    let card = aggregate_card_data(&fixture_data(), OutputKind::Languages);
    let wide = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(900, 260)
        .unwrap();
    let narrow = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(420, 240)
        .unwrap();

    let wide_svg = render_card(&card, &wide);
    let narrow_svg = render_card(&card, &narrow);

    assert!(
        !wide_svg.contains(r#"height="4""#),
        "wide layout drops per-row tracks in favour of two columns"
    );
    assert!(
        narrow_svg.contains(r#"height="4""#),
        "narrow layout keeps per-row tracks"
    );
    assert!(narrow_svg.contains(r#"text-anchor="end""#));
}

#[test]
fn narrow_streak_card_shrinks_ticks_and_drops_the_year_from_dates() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(420, 200)
        .unwrap();
    let svg = render_card(&sample_streak(vec![0, 2, 4]), &config);

    assert!(svg.contains(r#"stroke-width="3""#));
    assert!(!svg.contains(r#"stroke-width="3.6""#));
    assert!(svg.contains("May 20 – May 24"));
    assert!(!svg.contains("May 20 2026"));
}

#[test]
fn flat_recent_window_paints_every_active_day_at_full_heat() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(500, 220)
        .unwrap();
    let svg = render_card(&sample_streak(vec![1, 1, 0, 1]), &config);

    assert_eq!(svg.matches("#fb8c00").count(), 3);
    assert!(!svg.contains("#ffe3ad"));
}

#[test]
fn unparsable_dates_render_verbatim() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(420, 200)
        .unwrap();
    let mut streak = match sample_streak(vec![1]) {
        CardData::Streak(streak) => streak,
        _ => unreachable!(),
    };
    streak.current_start = Some("2026-13-40".to_owned());
    streak.current_end = Some("whenever".to_owned());

    let svg = render_card(&CardData::Streak(streak), &config);

    assert!(svg.contains("2026-13-40 – whenever"));
}

#[test]
fn status_badge_keeps_a_legible_foreground_on_every_theme() {
    for (theme, expected) in [
        ("default", "#ffffff"),
        ("transparent", "#ffffff"),
        ("dark", "#0d1117"),
    ] {
        let mut config = GithubStatsConfig::new("octo").unwrap();
        config.theme = theme.to_owned();

        let svg = render_card(&CardData::Status { state: "ready" }, &config);

        assert!(
            svg.contains(&format!(r#"fill="{expected}">ready<"#)),
            "{theme} theme must paint the badge label with {expected}"
        );
    }
}

fn sample_streak(recent_daily_counts: Vec<u32>) -> CardData {
    CardData::Streak(StreakSummary {
        current: 5,
        longest: 6,
        total_active_days: 6,
        total_contributions: 17,
        current_start: Some("2026-05-20".to_owned()),
        current_end: Some("2026-05-24".to_owned()),
        longest_start: Some("2026-04-01".to_owned()),
        longest_end: Some("2026-04-06".to_owned()),
        mode: StreakMode::Daily,
        recent_daily_counts,
    })
}

fn sample_stats(percentile_basis_points: u32) -> AggregatedStats {
    AggregatedStats {
        total_stars: 10,
        total_commits: 20,
        total_pull_requests: 30,
        total_issues: 40,
        total_reviews: 50,
        contributed_to: 60,
        score: 700,
        rank: "B",
        percentile_basis_points,
    }
}

fn dash_array(svg: &str) -> f64 {
    let marker = r#"stroke-dasharray=""#;
    let start = svg.find(marker).expect("ring dash array") + marker.len();
    let rest = &svg[start..];
    let end = rest.find(' ').expect("dash array length");
    rest[..end].parse().expect("numeric dash array")
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
        percentile_basis_points: 9_500,
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
    assert!(dark_svg.contains("#4493f8"));
    assert!(dark_svg.contains("#8b949e"));
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
