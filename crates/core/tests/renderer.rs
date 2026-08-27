use github_personal_stats_core::{
    ActivityComparison, ActivityMeasure, ActivitySpan, AggregatedStats, Author, BlockSpec,
    CardData, ChartRows, ChartStyle, ChartValue, CodingActivityEntry, DayBucket, GithubClient,
    GithubStatsConfig, GithubStatsError, HeatRing, LanguageShare, MEASURE_AGENT, MockGithubClient,
    OutputKind, StreakMode, StreakSummary, aggregate_card_data, aggregate_coding_activity,
    build_blocks, compare_activity, render_card, render_readme_section, render_text_chart,
};

/// A day of agent time spread over the given languages, with lines to match so
/// the card has both a duration and an authorship to draw.
fn worked_day(date: &str, languages: &[(&str, u64)]) -> DayBucket {
    let mut day = DayBucket::new(date);
    let bucket = day.measure_mut(MEASURE_AGENT);
    for (language, seconds) in languages {
        bucket.seconds += seconds;
        bucket.languages.insert((*language).to_owned(), *seconds);
    }
    bucket.sessions = 1;
    for (language, seconds) in languages {
        day.add_lines(language, Author::Agent, "composer-2.5", seconds / 10, 0);
    }
    day
}

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
    let card = aggregate_card_data(&fixture_data(), OutputKind::Dashboard, &HeatRing::default());
    let svg = render_card(&card, &config);

    assert_eq!(svg, DASHBOARD_SNAPSHOT.trim_end());
}

#[test]
fn stats_renderer_matches_snapshot() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(420, 220)
        .unwrap();
    let card = aggregate_card_data(&fixture_data(), OutputKind::Stats, &HeatRing::default());
    let svg = render_card(&card, &config);

    assert_eq!(svg, STATS_SNAPSHOT.trim_end());
}

#[test]
fn every_card_names_itself_so_a_screen_reader_can_describe_it() {
    let config = GithubStatsConfig::new("octo").unwrap();
    let data = fixture_data();
    let named = [
        (OutputKind::Dashboard, "GitHub profile summary for octo"),
        (OutputKind::Stats, "GitHub stats for octo"),
        (OutputKind::Languages, "Top languages for octo"),
        (OutputKind::Streak, "Contribution streak for octo"),
    ];

    for (kind, expected) in named {
        let card = aggregate_card_data(&data, kind, &HeatRing::default());
        let svg = render_card(&card, &config);

        assert!(
            svg.contains(r#"role="img" aria-labelledby="gps-title""#),
            "{kind:?} left role=img without an accessible name"
        );
        assert!(
            svg.contains(&format!(r#"<title id="gps-title">{expected}</title>"#)),
            "{kind:?} did not describe itself as {expected}"
        );
    }
}

#[test]
fn a_username_carrying_markup_cannot_break_out_of_the_title() {
    let config = GithubStatsConfig::new("a&b<script>").unwrap();
    let card = aggregate_card_data(&fixture_data(), OutputKind::Stats, &HeatRing::default());
    let svg = render_card(&card, &config);

    assert!(svg.contains("GitHub stats for a&amp;b&lt;script&gt;"));
    assert!(!svg.contains("<script>"));
}

#[test]
fn renderer_sets_fixed_svg_dimensions() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(700, 300)
        .unwrap();
    let card = aggregate_card_data(&fixture_data(), OutputKind::Languages, &HeatRing::default());
    let svg = render_card(&card, &config);

    assert!(svg.contains(r#"width="700""#));
    assert!(svg.contains(r#"height="300""#));
    assert!(svg.contains(r#"viewBox="0 0 700 300""#));
}

#[test]
fn language_renderer_uses_language_specific_colors() {
    let config = GithubStatsConfig::new("octo").unwrap();
    let card = aggregate_card_data(&fixture_data(), OutputKind::Languages, &HeatRing::default());
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
        recent_daily_counts: vec![0, 1, 2, 3, 4],
        window_start: Some("2026-05-22".to_owned()),
        window_end: Some("2026-05-24".to_owned()),
    });
    let activity = CardData::Activity(Box::new(compare_activity(
        &[worked_day(
            "2026-05-24",
            &[("Rust", 3660), ("TypeScript", 7200)],
        )],
        ActivityMeasure::default(),
        [ActivitySpan::Days(30), ActivitySpan::All],
        Some("2026-05-24"),
        5,
        &[],
    )));
    let status = CardData::Status { state: "ready" };

    let streak_svg = render_card(&streak, &config);
    let activity_svg = render_card(&activity, &config);
    let status_svg = render_card(&status, &config);

    assert!(streak_svg.contains("Current Streak"));
    assert!(streak_svg.contains("May 22 2026 – May 24 2026"));
    assert!(streak_svg.contains("1,234"));
    assert!(activity_svg.contains("CODING ACTIVITY"));
    assert!(activity_svg.contains(">TypeScript<"));
    // The card names the measure it is reporting, because a day holds several and
    // they overlap.
    assert!(activity_svg.contains("AGENT TIME"));
    assert!(activity_svg.contains(">3 hrs 1 mins<"));
    assert!(activity_svg.contains(">1 day active<"));
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
        window_start: Some("2026-05-22".to_owned()),
        window_end: Some("2026-05-24".to_owned()),
    });

    let svg = render_card(&streak, &config);

    assert!(!svg.contains(r#"stroke-width="3.6""#));
    assert!(svg.contains(r##"stroke="#e4e8ee" stroke-width="3""##));
}

#[test]
fn language_rows_switch_layout_with_available_width() {
    let card = aggregate_card_data(&fixture_data(), OutputKind::Languages, &HeatRing::default());
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
fn stacked_language_bar_sizes_every_segment_by_its_own_share() {
    let shares = (0..6)
        .map(|index| LanguageShare {
            name: format!("Lang{index}"),
            size: 100,
            percentage_basis_points: 1_000,
        })
        .collect::<Vec<_>>();
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(1000, 420)
        .unwrap();

    let svg = render_card(&CardData::Languages(shares), &config);
    let widths = stacked_bar_widths(&svg);

    assert_eq!(widths.len(), 6);
    assert!(
        widths.iter().all(|width| (94..=95).contains(width)),
        "each segment must be its own 10% of the 944 wide bar, got {widths:?}"
    );
    assert_eq!(
        widths.iter().sum::<u32>(),
        566,
        "the 40% held by unlisted languages must stay on the track"
    );
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
    streak.window_start = Some("2026-13-40".to_owned());
    streak.window_end = Some("whenever".to_owned());

    let svg = render_card(&CardData::Streak(streak), &config);

    assert!(svg.contains("2026-13-40 – whenever"));
}

#[test]
fn status_badge_keeps_a_legible_foreground_on_every_theme() {
    for (theme, expected) in [
        ("light", "#ffffff"),
        ("transparent", "#ffffff"),
        ("dark", "#0d1117"),
    ] {
        let config = GithubStatsConfig::new("octo")
            .unwrap()
            .with_theme(theme)
            .unwrap();

        let svg = render_card(&CardData::Status { state: "ready" }, &config);

        assert!(
            svg.contains(&format!(r#"fill="{expected}">ready<"#)),
            "{theme} theme must paint the badge label with {expected}"
        );
    }
}

fn stacked_bar_widths(svg: &str) -> Vec<u32> {
    let marker = r#"<g clip-path="url(#gps-language-bar)">"#;
    let start = svg.find(marker).expect("stacked bar group") + marker.len();
    let group = &svg[start..start + svg[start..].find("</g>").expect("group end")];

    group
        .split(r#"width=""#)
        .skip(1)
        .map(|part| {
            part[..part.find('"').expect("width value")]
                .parse()
                .expect("numeric width")
        })
        .collect()
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
        window_start: Some("2026-05-20".to_owned()),
        window_end: Some("2026-05-24".to_owned()),
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
    let dark_config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_theme("dark")
        .unwrap();
    let transparent_config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_theme("transparent")
        .unwrap();

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
fn theme_names_are_checked_instead_of_falling_back_to_light() {
    for name in ["light", "LIGHT", " dark ", "transparent", "default"] {
        assert!(
            GithubStatsConfig::new("octo")
                .unwrap()
                .with_theme(name)
                .is_ok(),
            "{name} must be an accepted theme"
        );
    }

    let error = GithubStatsConfig::new("octo")
        .unwrap()
        .with_theme("drak")
        .expect_err("a misspelled theme must not render as light");

    assert!(matches!(
        error,
        GithubStatsError::InvalidConfig { field: "theme", .. }
    ));
}

#[test]
fn light_and_dark_cards_differ_only_by_their_palette() {
    let light = render_card(
        &CardData::Status { state: "ready" },
        &GithubStatsConfig::new("octo")
            .unwrap()
            .with_theme("light")
            .unwrap(),
    );
    let dark = render_card(
        &CardData::Status { state: "ready" },
        &GithubStatsConfig::new("octo")
            .unwrap()
            .with_theme("dark")
            .unwrap(),
    );

    assert!(light.contains(r##"fill="#ffffff""##));
    assert!(dark.contains(r##"fill="#0d1117""##));
    assert!(!dark.contains(r##"fill="#ffffff""##));
    assert_eq!(
        light.matches("<text").count(),
        dark.matches("<text").count(),
        "a theme must repaint the card, not change what it says"
    );
}

#[test]
fn a_card_can_be_set_in_the_face_a_fenced_block_is_set_in() {
    let card = |font: Option<&str>| {
        let mut config = GithubStatsConfig::new("octo").unwrap();
        if let Some(font) = font {
            config = config.with_typeface(font).unwrap();
        }
        render_card(&CardData::Status { state: "ready" }, &config)
    };

    let sans = card(None);
    let mono = card(Some("mono"));

    assert!(sans.contains("sans-serif"), "{sans}");
    assert!(!sans.contains("monospace"), "{sans}");
    assert!(mono.contains("ui-monospace"), "{mono}");
    assert!(!mono.contains("sans-serif"), "{mono}");
    // A face is a face: asking for one must not move anything or say anything
    // different, since the layout measures a character the same either way.
    assert_eq!(sans.matches("<text").count(), mono.matches("<text").count());
    assert_eq!(card(Some("sans")), sans);

    for name in ["mono", "MONO", " monospace ", "sans", "ui", "default"] {
        assert!(
            GithubStatsConfig::new("octo")
                .unwrap()
                .with_typeface(name)
                .is_ok(),
            "{name} must name a face"
        );
    }

    let error = GithubStatsConfig::new("octo")
        .unwrap()
        .with_typeface("comic")
        .expect_err("a face nobody has must not quietly become the default");

    assert!(matches!(
        error,
        GithubStatsError::InvalidConfig { field: "font", .. }
    ));
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

/// A day where a source knew how long the work took but never what it was, which
/// is every terminal agent, and on a real record the largest share of all.
fn unplaced_day(date: &str, seconds: u64) -> DayBucket {
    let mut day = DayBucket::new(date);
    let bucket = day.measure_mut(MEASURE_AGENT);
    bucket.seconds = seconds;
    bucket.sessions = 1;
    day
}

fn comparison_over(days: &[DayBucket]) -> ActivityComparison {
    compare_activity(
        days,
        ActivityMeasure::default(),
        [ActivitySpan::Days(30), ActivitySpan::All],
        Some("2026-05-24"),
        8,
        &[],
    )
}

#[test]
fn time_no_language_can_be_put_to_is_declared_rather_than_ranked() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(760, 260)
        .unwrap();
    let comparison = comparison_over(&[
        worked_day("2026-05-24", &[("Rust", 3600)]),
        unplaced_day("2026-05-23", 7200),
    ]);

    let svg = render_card(&CardData::Activity(Box::new(comparison)), &config);

    // The nameless share used to sort first and draw a bar with no label beside
    // it, which is how it was found.
    assert!(svg.contains(">Rust<"));
    assert!(!svg.contains("font-size=\"11.5\" font-weight=\"400\" fill=\"#15181d\"></text>"));
    assert!(svg.contains("2 hrs 0 mins not placed to a language"));
    // Rust is all of the time that could be placed, even though it is a third of
    // the time measured.
    assert!(svg.contains(">100.0%<"));
}

#[test]
fn a_card_and_a_chart_of_the_same_measure_give_the_same_share() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(760, 260)
        .unwrap();
    let comparison = comparison_over(&[
        worked_day("2026-05-24", &[("Rust", 3600), ("Go", 1200)]),
        unplaced_day("2026-05-23", 9000),
    ]);

    let svg = render_card(&CardData::Activity(Box::new(comparison.clone())), &config);
    let spec = BlockSpec::new(ChartValue::Time, ChartRows::Languages);
    let chart = render_text_chart(
        &build_blocks(std::slice::from_ref(&comparison), &[spec]),
        &ChartStyle::default(),
    );

    // Both read hours by language off the same fold, so a reader putting the card
    // beside the chart must not find two different numbers for Rust.
    assert!(svg.contains(">75.0%<"), "card: {svg}");
    assert!(chart.contains("75.00 %"), "chart: {chart}");
}

#[test]
fn spans_too_long_to_share_a_line_are_given_one_each() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(275, 260)
        .unwrap()
        .with_auto_height();
    // Six hundred hours, which is what a record a few months old holds, and long
    // enough that two of these cannot share the width of a tile.
    let comparison = comparison_over(&[
        worked_day("2026-05-24", &[("Rust", 360_000)]),
        worked_day("2026-04-24", &[("Rust", 1_800_000)]),
    ]);

    let svg = render_card(&CardData::Activity(Box::new(comparison)), &config);

    let baselines = leading_baselines(&svg);
    assert_eq!(baselines.len(), 2, "both spans are named: {svg}");
    assert_ne!(
        baselines[0], baselines[1],
        "a tile wrote one span's figure across the other: {svg}"
    );
    // The card has to grow to hold the second row rather than crop it.
    let note = "not placed to a language";
    assert!(!svg.contains(note), "nothing here is unplaced: {svg}");
    assert!(svg.contains(">Rust<"), "the language survived: {svg}");
}

/// Where each span's figure sits, read off the size only a leading figure uses.
fn leading_baselines(svg: &str) -> Vec<String> {
    svg.split("<text ")
        .filter(|element| element.contains("hrs "))
        .filter_map(|element| {
            let y = element.split("y=\"").nth(1)?.split('"').next()?;
            Some(y.to_owned())
        })
        .collect()
}
