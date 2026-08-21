use github_personal_stats_core::{
    CardData, GithubClient, GithubStatsConfig, HeatRing, LanguageShare, MAX_LANGUAGE_ROWS,
    MockGithubClient, OutputKind, StatMetric, StreakMetric, aggregate_card_data, render_card,
};

const FIXTURE: &str = include_str!("fixtures/github_user_data.json");

fn fixture_card(kind: OutputKind) -> CardData {
    let config = GithubStatsConfig::new("octo").unwrap();
    let data = MockGithubClient::success(FIXTURE)
        .fetch_user_data(&config)
        .unwrap();
    aggregate_card_data(&data, kind, &HeatRing::default())
}

fn labels(svg: &str) -> Vec<String> {
    svg.split("</text>")
        .filter_map(|chunk| chunk.rsplit_once('>').map(|(_, text)| text.to_owned()))
        .filter(|text| !text.is_empty())
        .collect()
}

#[test]
fn the_default_panels_keep_the_rows_the_card_has_always_shown() {
    let config = GithubStatsConfig::new("octo").unwrap();

    assert_eq!(
        config.stat_rows,
        vec![
            StatMetric::Stars,
            StatMetric::Commits,
            StatMetric::PullRequests,
            StatMetric::Issues
        ]
    );
    assert_eq!(config.language_rows, 6);
    assert_eq!(
        config.streak_sides,
        [
            StreakMetric::TotalContributions,
            StreakMetric::LongestStreak
        ]
    );
}

#[test]
fn reviews_and_repositories_reach_the_card_once_they_are_asked_for() {
    let config = GithubStatsConfig::new("octo").unwrap();
    let card = fixture_card(OutputKind::Stats);
    let default = render_card(&card, &config);

    assert!(!default.contains("Reviews"));
    assert!(!default.contains("Contributed To"));

    let config = config.with_stat_rows("reviews,repos").unwrap();
    let svg = render_card(&card, &config);
    let rows = labels(&svg);

    assert!(rows.contains(&"Reviews".to_owned()));
    assert!(rows.contains(&"Contributed To".to_owned()));
    assert!(!rows.contains(&"Total Stars".to_owned()));
}

#[test]
fn the_stats_rows_appear_in_the_order_they_were_named() {
    let card = fixture_card(OutputKind::Stats);
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_stat_rows("issues,stars,reviews")
        .unwrap();
    let svg = render_card(&card, &config);
    let ordered = labels(&svg)
        .into_iter()
        .filter(|row| ["Issues", "Total Stars", "Reviews"].contains(&row.as_str()))
        .collect::<Vec<_>>();

    assert_eq!(ordered, ["Issues", "Total Stars", "Reviews"]);
}

/// The row step divides the space the panel has, so a longer list packs tighter,
/// but never below the floor that keeps two rows from touching.
#[test]
fn a_crowded_stats_list_packs_tighter_without_dropping_below_the_readable_floor() {
    let card = fixture_card(OutputKind::Stats);
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(1000, 240)
        .unwrap();
    let two = render_card(
        &card,
        &config.clone().with_stat_rows("stars,commits").unwrap(),
    );
    let six = render_card(
        &card,
        &config
            .with_stat_rows("stars,commits,prs,issues,reviews,repos")
            .unwrap(),
    );

    assert!(
        row_step(&two) > row_step(&six),
        "two rows should sit further apart than six on the same card"
    );
    assert!(
        row_step(&six) >= 20,
        "six rows should still keep the minimum spacing, got {}",
        row_step(&six)
    );
}

fn row_step(svg: &str) -> u32 {
    let baselines = svg
        .split("<text")
        .filter(|chunk| chunk.contains(r#"font-size="12.5" font-weight="400""#))
        .filter_map(|chunk| {
            chunk
                .split_once(r#"y=""#)
                .and_then(|(_, rest)| rest.split_once('"'))
                .and_then(|(value, _)| value.parse::<u32>().ok())
        })
        .collect::<Vec<_>>();

    baselines
        .windows(2)
        .map(|pair| pair[1].saturating_sub(pair[0]))
        .next()
        .unwrap_or(0)
}

#[test]
fn the_language_panel_lists_exactly_the_rows_it_was_given() {
    let card = fixture_card(OutputKind::Languages);
    let config = GithubStatsConfig::new("octo").unwrap();
    let CardData::Languages(available) = &card else {
        panic!("expected a languages card");
    };

    for rows in 1..=available.len() {
        let svg = render_card(
            &card,
            &config
                .clone()
                .with_language_rows(&rows.to_string())
                .unwrap(),
        );
        let listed = available
            .iter()
            .filter(|language| svg.contains(&format!(">{}<", language.name)))
            .count();

        assert_eq!(listed, rows, "asked for {rows} language rows");
    }
}

#[test]
fn an_odd_language_count_still_fills_two_columns() {
    let card = fixture_card(OutputKind::Languages);
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(700, 260)
        .unwrap()
        .with_language_rows("3")
        .unwrap();
    let svg = render_card(&card, &config);
    let columns = language_dot_columns(&svg);

    assert_eq!(
        columns.len(),
        2,
        "three rows should split across two columns"
    );
}

fn language_dot_columns(svg: &str) -> Vec<u32> {
    let mut columns = svg
        .split("<circle")
        .filter(|chunk| chunk.contains(r#"r="4""#))
        .filter_map(|chunk| {
            chunk
                .split_once(r#"cx=""#)
                .and_then(|(_, rest)| rest.split_once('"'))
                .and_then(|(value, _)| value.parse::<u32>().ok())
        })
        .collect::<Vec<_>>();
    columns.sort_unstable();
    columns.dedup();
    columns
}

#[test]
fn either_streak_panel_can_report_any_of_the_four_figures() {
    let card = fixture_card(OutputKind::Streak);
    let config = GithubStatsConfig::new("octo").unwrap();
    let svg = render_card(&card, &config.with_streak_sides("active,current").unwrap());
    let rows = labels(&svg);

    assert!(rows.contains(&"Active Days".to_owned()));
    assert!(rows.contains(&"Current Streak".to_owned()));
    assert!(!rows.contains(&"Total Contributions".to_owned()));
    assert!(!rows.contains(&"Longest Streak".to_owned()));
}

#[test]
fn a_streak_panel_dates_the_figure_above_it() {
    let card = fixture_card(OutputKind::Streak);
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_streak_sides("longest,total")
        .unwrap();
    let svg = render_card(&card, &config);
    let CardData::Streak(streak) = &card else {
        panic!("expected a streak card");
    };
    let longest_panel = svg.split("Longest Streak").nth(1).unwrap();
    let total_panel = svg.split("Total Contributions").nth(1).unwrap();
    let longest_year = &streak.longest_start.as_deref().unwrap()[..4];

    assert!(
        longest_panel.contains(longest_year),
        "the longest panel should date the streak it reports"
    );
    assert!(
        total_panel.contains("through"),
        "the total panel should say how current it is"
    );
}

#[test]
fn a_panel_refuses_a_list_it_cannot_draw() {
    let config = GithubStatsConfig::new("octo").unwrap();

    for rejected in ["", "   ", "stars,stars", "nonsense", ","] {
        assert!(
            config.clone().with_stat_rows(rejected).is_err(),
            "{rejected:?} should be refused"
        );
    }

    assert!(config.clone().with_stat_rows("stars,,commits").is_ok());
    assert!(config.clone().with_streak_sides("total").is_err());
    assert!(
        config
            .clone()
            .with_streak_sides("total,longest,current")
            .is_err()
    );
    assert!(config.clone().with_streak_sides("total,total").is_err());
    assert!(config.clone().with_language_rows("0").is_err());
    assert!(
        config
            .clone()
            .with_language_rows(&(MAX_LANGUAGE_ROWS + 1).to_string())
            .is_err()
    );
    assert!(config.clone().with_language_rows("two").is_err());
    assert!(
        config
            .with_language_rows(&MAX_LANGUAGE_ROWS.to_string())
            .is_ok()
    );
}

#[test]
fn every_stats_row_carries_an_icon_of_its_own() {
    let card = fixture_card(OutputKind::Stats);
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_stat_rows("stars,commits,prs,issues,reviews,repos")
        .unwrap();
    let svg = render_card(&card, &config);
    let mut icons = svg
        .split(r#"aria-hidden="true">"#)
        .skip(1)
        .filter_map(|chunk| chunk.split_once("</svg>").map(|(markup, _)| markup))
        .collect::<Vec<_>>();
    let drawn = icons.len();
    icons.sort_unstable();
    icons.dedup();

    assert_eq!(drawn, 6, "six rows should draw six icons");
    assert_eq!(icons.len(), 6, "each row should draw a distinct icon");
}

#[test]
fn unused_language_shares_leave_the_bar_track_showing() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_language_rows("1")
        .unwrap();
    let card = CardData::Languages(vec![
        LanguageShare {
            name: "Rust".to_owned(),
            size: 600,
            percentage_basis_points: 6_000,
        },
        LanguageShare {
            name: "Go".to_owned(),
            size: 400,
            percentage_basis_points: 4_000,
        },
    ]);
    let svg = render_card(&card, &config);

    assert!(svg.contains(">Rust<"));
    assert!(!svg.contains(">Go<"));
}
