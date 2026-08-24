use github_personal_stats_core::{
    CardData, GithubClient, GithubStatsConfig, HeatRing, MockGithubClient, OutputKind, StatMetric,
    StreakMetric, TileMetric, aggregate_card_data, parse_output_kind, render_card,
};

const FIXTURE: &str = include_str!("fixtures/github_user_data.json");

fn tile(kind: OutputKind, metric: Option<&str>) -> String {
    let mut config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(275, 140)
        .unwrap();
    if let Some(metric) = metric {
        config = config.with_metric(metric).unwrap();
    }
    let data = MockGithubClient::success(FIXTURE)
        .fetch_user_data(&config)
        .unwrap();
    let card = aggregate_card_data(&data, kind, &HeatRing::default());
    render_card(&card, &config)
}

fn centred_baselines(svg: &str) -> Vec<u32> {
    svg.split("<text")
        .skip(1)
        .filter(|chunk| chunk.contains(r#"text-anchor="middle""#))
        .filter_map(|chunk| {
            chunk
                .split_once(r#"y=""#)
                .and_then(|(_, rest)| rest.split_once('"'))
                .and_then(|(value, _)| value.parse().ok())
        })
        .collect()
}

#[test]
fn every_single_metric_name_the_panels_accept_also_works_on_its_own_tile() {
    for name in [
        "stars", "commits", "prs", "issues", "reviews", "repos", "total", "longest", "current",
        "active",
    ] {
        let svg = tile(OutputKind::Metric, Some(name));
        assert!(
            svg.contains("<text"),
            "the {name} tile should draw its figure"
        );
    }
}

#[test]
fn a_tile_metric_reuses_the_names_the_panel_lists_already_use() {
    assert_eq!(
        TileMetric::parse("stars").unwrap(),
        TileMetric::Stat(StatMetric::Stars)
    );
    assert_eq!(
        TileMetric::parse("longest").unwrap(),
        TileMetric::Streak(StreakMetric::LongestStreak)
    );
    assert!(TileMetric::parse("nonsense").is_err());
}

#[test]
fn a_single_figure_sits_centred_rather_than_hugging_an_edge() {
    let svg = tile(OutputKind::Metric, Some("total"));

    // Label and note are centred outright; the value and its unit are centred as
    // a group, so at least the two captions must be anchored to the middle.
    assert!(
        centred_baselines(&svg).len() >= 2,
        "a standalone figure should centre its captions"
    );
}

#[test]
fn a_heat_tile_draws_the_ring_without_a_section_heading() {
    let svg = tile(OutputKind::Heat, None);

    assert!(svg.contains("<path"), "the ring should be drawn");
    assert!(
        !svg.contains("STREAK"),
        "a ring on its own needs no section eyebrow"
    );
}

#[test]
fn a_tile_names_the_figure_it_reports_so_a_screen_reader_can_tell_them_apart() {
    let stars = tile(OutputKind::Metric, Some("stars"));
    let longest = tile(OutputKind::Metric, Some("longest"));

    assert!(stars.contains("<title id=\"gps-title\">Total Stars for octo</title>"));
    assert!(longest.contains("<title id=\"gps-title\">Longest Streak for octo</title>"));
}

#[test]
fn the_new_card_names_are_selectable_from_the_command_line() {
    assert_eq!(parse_output_kind("heat").unwrap(), OutputKind::Heat);
    assert_eq!(parse_output_kind("ring").unwrap(), OutputKind::Heat);
    assert_eq!(parse_output_kind("metric").unwrap(), OutputKind::Metric);
}

#[test]
fn a_tile_keeps_its_figure_inside_the_canvas() {
    let svg = tile(OutputKind::Metric, Some("total"));
    let lowest = svg
        .split("<text")
        .skip(1)
        .filter_map(|chunk| {
            chunk
                .split_once(r#"y=""#)
                .and_then(|(_, rest)| rest.split_once('"'))
                .and_then(|(value, _)| value.parse::<u32>().ok())
        })
        .max()
        .expect("the tile should draw text");

    assert!(
        lowest < 140,
        "text at y={lowest} would fall off a 140px tile"
    );
}

#[test]
fn a_metric_card_is_the_only_card_that_reads_the_metric_setting() {
    let streak = tile(OutputKind::Streak, Some("stars"));

    assert!(
        streak.contains("Total Contributions"),
        "the streak card should keep its own panels regardless of --metric"
    );
    assert!(matches!(
        aggregate_card_data(
            &MockGithubClient::success(FIXTURE)
                .fetch_user_data(&GithubStatsConfig::new("octo").unwrap())
                .unwrap(),
            OutputKind::Heat,
            &HeatRing::default()
        ),
        CardData::Heat(_)
    ));
}
