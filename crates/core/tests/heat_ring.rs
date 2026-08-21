use github_personal_stats_core::{
    CardData, ContributionDay, GithubStatsConfig, HeatRing, StreakMode, calculate_streak,
    parse_heat_ramp, render_card,
};

const TICK: &str = r#"stroke-width="3.6" stroke-linecap="round""#;
const ARC: &str = r#"stroke-width="10""#;

fn config() -> GithubStatsConfig {
    GithubStatsConfig::new("octo")
        .unwrap()
        .with_cards("streak")
        .unwrap()
        .with_size(1000, 220)
        .unwrap()
}

fn streak_days(days: u32, count_at: impl Fn(u32) -> u32) -> Vec<ContributionDay> {
    let start = 800_000i64;
    (0..days)
        .map(|offset| ContributionDay {
            date: ordinal_to_date(start + i64::from(offset)),
            count: count_at(offset),
        })
        .collect()
}

fn ordinal_to_date(ordinal: i64) -> String {
    let days = ordinal - 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year + i64::from(month <= 2);

    format!("{year:04}-{month:02}-{day:02}")
}

fn render(contributions: &[ContributionDay], ring: HeatRing) -> String {
    let mut config = config();
    config.heat_ring = ring;
    let streak = calculate_streak(contributions, StreakMode::Daily, &[], &config.heat_ring);

    render_card(&CardData::Streak(streak), &config)
}

fn count(svg: &str, marker: &str) -> usize {
    svg.matches(marker).count()
}

fn centre_number(svg: &str) -> String {
    let anchor = r##"font-weight="600" fill="#15181d" text-anchor="middle""##;
    let start = svg.find(anchor).expect("centre text");
    let opened = svg[start..].find('>').expect("text open") + start + 1;
    let closed = svg[opened..].find('<').expect("text close") + opened;

    svg[opened..closed].to_owned()
}

#[test]
fn streak_window_draws_one_tick_per_day_and_reports_that_same_count() {
    let svg = render(
        &streak_days(13, |offset| offset % 4 + 1),
        HeatRing::default(),
    );

    assert_eq!(count(&svg, TICK), 13);
    assert_eq!(centre_number(&svg), "13");
}

#[test]
fn a_streak_past_the_threshold_switches_to_arcs_without_losing_the_day_count() {
    let svg = render(
        &streak_days(117, |offset| offset % 7 + 1),
        HeatRing::default(),
    );

    assert_eq!(count(&svg, TICK), 0, "ticks stop being separable here");
    assert!(count(&svg, ARC) > 1, "the ring is drawn as arcs instead");
    assert_eq!(
        centre_number(&svg),
        "117",
        "the centre still reports every day of the streak"
    );
}

#[test]
fn arcs_are_averaged_into_bands_wide_enough_to_read() {
    let svg = render(
        &streak_days(365, |offset| offset % 5 + 1),
        HeatRing::default(),
    );

    let arcs = count(&svg, ARC);
    assert!(
        (30..=60).contains(&arcs),
        "365 days should collapse into readable bands, got {arcs}"
    );
    assert_eq!(centre_number(&svg), "365");
}

#[test]
fn an_explicit_shape_overrides_the_threshold_in_both_directions() {
    let ticks = render(
        &streak_days(117, |_| 3),
        HeatRing {
            shape: github_personal_stats_core::HeatShape::Ticks,
            ..HeatRing::default()
        },
    );
    let arcs = render(
        &streak_days(13, |_| 3),
        HeatRing {
            shape: github_personal_stats_core::HeatShape::Arcs,
            ..HeatRing::default()
        },
    );

    assert_eq!(count(&ticks, TICK), 117);
    assert_eq!(count(&arcs, ARC), 13, "one arc per day, no averaging");
}

#[test]
fn the_threshold_itself_is_configurable() {
    let svg = render(
        &streak_days(13, |_| 3),
        HeatRing {
            threshold: 10,
            ..HeatRing::default()
        },
    );

    assert_eq!(count(&svg, TICK), 0);
    assert!(count(&svg, ARC) > 1);
}

#[test]
fn a_fixed_window_keeps_its_length_and_marks_quiet_days_with_the_track() {
    let mut contributions = streak_days(30, |offset| if offset % 3 == 0 { 0 } else { 4 });
    contributions.truncate(30);
    let svg = render(
        &contributions,
        HeatRing {
            window: github_personal_stats_core::HeatWindow::Fixed(30),
            ..HeatRing::default()
        },
    );

    assert_eq!(count(&svg, TICK), 30, "quiet days still take a slot");
    assert!(
        svg.contains(r##"stroke="#e4e8ee""##),
        "quiet days fall back to the track colour"
    );
    assert_eq!(
        centre_number(&svg),
        "20/last 30",
        "the default fixed label counts active days against the window"
    );
}

#[test]
fn a_fixed_window_says_what_it_covers_instead_of_calling_itself_a_streak() {
    let fixed = render(
        &streak_days(40, |_| 3),
        HeatRing {
            window: github_personal_stats_core::HeatWindow::Fixed(30),
            ..HeatRing::default()
        },
    );
    let streak = render(&streak_days(40, |_| 3), HeatRing::default());

    assert!(fixed.contains("Last 30 Days"));
    assert!(!fixed.contains("Current Streak"));
    assert!(streak.contains("Current Streak"));
}

#[test]
fn a_limit_shortens_the_ring_while_the_streak_keeps_its_real_length() {
    let svg = render(
        &streak_days(117, |_| 3),
        HeatRing {
            limit: Some(30),
            label: Some("{Y} of {Z}".to_owned()),
            ..HeatRing::default()
        },
    );

    assert_eq!(count(&svg, TICK), 30);
    assert_eq!(centre_number(&svg), "30 of 117");
}

#[test]
fn the_label_template_fills_every_placeholder() {
    let svg = render(
        &streak_days(20, |offset| u32::from(offset % 2 == 0)),
        HeatRing {
            window: github_personal_stats_core::HeatWindow::Fixed(20),
            label: Some("{X} active in {Y} · best {Z}".to_owned()),
            ..HeatRing::default()
        },
    );

    assert_eq!(centre_number(&svg), "10 active in 20 · best 1");
}

#[test]
fn a_long_label_shrinks_instead_of_spilling_over_the_ring() {
    let size = |label: &str| {
        let svg = render(
            &streak_days(13, |_| 3),
            HeatRing {
                label: Some(label.to_owned()),
                ..HeatRing::default()
            },
        );
        let anchor = svg
            .find(r##"font-weight="600" fill="#15181d" text-anchor="middle""##)
            .expect("centre text");
        let start = svg[..anchor].rfind(r#"font-size=""#).expect("size") + 11;
        let end = svg[start..].find('"').expect("size end") + start;
        svg[start..end].parse::<f64>().expect("numeric size")
    };

    assert_eq!(size("{Y}"), 26.0, "a bare count keeps the full size");
    let long = size("{X} active in {Y} · best {Z}");
    assert!(
        (9.0..26.0).contains(&long),
        "long labels step down, got {long}"
    );
}

#[test]
fn a_quantile_scale_spreads_a_heavy_tail_across_the_whole_ramp() {
    let contributions = streak_days(
        40,
        |offset| if offset % 13 == 0 { 60 } else { offset % 3 + 1 },
    );
    let ramp = parse_heat_ramp("heat-orange").unwrap();
    let shades = |svg: &str| {
        ramp.iter()
            .filter(|stop| svg.contains(stop.as_str()))
            .count()
    };

    let linear = render(&contributions, HeatRing::default());
    let quantile = render(
        &contributions,
        HeatRing {
            scale: github_personal_stats_core::HeatScale::Quantile,
            ..HeatRing::default()
        },
    );

    assert_eq!(
        shades(&linear),
        2,
        "a linear scale pins the quiet majority to the lightest stop"
    );
    assert_eq!(
        shades(&quantile),
        4,
        "a quantile scale uses every stop instead"
    );
}

#[test]
fn a_named_palette_repaints_the_ring() {
    let svg = render(
        &streak_days(13, |offset| offset % 4 + 1),
        HeatRing {
            ramp: parse_heat_ramp("github-blue").unwrap(),
            ..HeatRing::default()
        },
    );

    assert!(svg.contains("#0b69d4"));
    assert!(!svg.contains("#fb8c00"), "the orange default is gone");
}

#[test]
fn heat_orange_stays_the_default() {
    assert_eq!(
        HeatRing::default().ramp,
        vec!["#ffe3ad", "#ffc65c", "#ffa726", "#fb8c00"]
    );
}

#[test]
fn one_colour_derives_a_ramp_that_only_deepens() {
    let ramp = parse_heat_ramp("#fb8c00").unwrap();

    assert_eq!(ramp.len(), 4);
    assert_eq!(ramp.last().map(String::as_str), Some("#fb8c00"));
    let luminance = ramp
        .iter()
        .map(|stop| {
            let channel = |start: usize| {
                u32::from_str_radix(&stop[start..start + 2], 16).expect("hex channel")
            };
            channel(1) + channel(3) + channel(5)
        })
        .collect::<Vec<_>>();
    assert!(
        luminance.windows(2).all(|pair| pair[0] > pair[1]),
        "each stop must be darker than the one before it, got {luminance:?}"
    );
}

#[test]
fn four_colours_are_taken_verbatim() {
    let ramp = parse_heat_ramp("#111111, #222222,#333333 , #444444").unwrap();

    assert_eq!(ramp, vec!["#111111", "#222222", "#333333", "#444444"]);
}

#[test]
fn a_palette_needs_a_name_one_colour_or_four() {
    assert!(parse_heat_ramp("#111111,#222222").is_err());
    assert!(parse_heat_ramp("mauve").is_err());
    assert!(parse_heat_ramp("#12345").is_err());
    assert!(parse_heat_ramp("").is_err());
}

#[test]
fn a_window_with_no_activity_at_all_still_draws() {
    for scale in [
        github_personal_stats_core::HeatScale::Linear,
        github_personal_stats_core::HeatScale::Sqrt,
        github_personal_stats_core::HeatScale::Log,
        github_personal_stats_core::HeatScale::Quantile,
    ] {
        let svg = render(
            &streak_days(10, |_| 0),
            HeatRing {
                window: github_personal_stats_core::HeatWindow::Fixed(10),
                scale,
                ..HeatRing::default()
            },
        );

        assert_eq!(count(&svg, TICK), 10, "{scale:?} draws every quiet day");
        assert_eq!(centre_number(&svg), "0/last 10");
    }
}

#[test]
fn an_empty_streak_falls_back_to_a_plain_track() {
    let svg = render(&[], HeatRing::default());

    assert_eq!(count(&svg, TICK), 0);
    assert!(svg.contains(r##"fill="none" stroke="#e4e8ee" stroke-width="3""##));
    assert_eq!(centre_number(&svg), "0");
}

#[test]
fn ring_options_reject_values_that_cannot_be_drawn() {
    let base = || GithubStatsConfig::new("octo").unwrap();

    assert!(base().with_heat_window("0").is_err());
    assert!(base().with_heat_window("last-week").is_err());
    assert!(base().with_heat_limit("-4").is_err());
    assert!(base().with_heat_threshold("0").is_err());
    assert!(base().with_heat_shape("spiral").is_err());
    assert!(base().with_heat_scale("bezier").is_err());
    assert!(base().with_heat_color("chartreuse").is_err());
}

#[test]
fn ring_options_accept_the_documented_spellings() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_heat_window("streak")
        .unwrap()
        .with_heat_limit("none")
        .unwrap()
        .with_heat_shape("bands")
        .unwrap()
        .with_heat_threshold("87")
        .unwrap()
        .with_heat_scale("sqrt")
        .unwrap()
        .with_heat_color("forest")
        .unwrap()
        .with_heat_label("{X}/{Y}");

    assert_eq!(config.heat_ring.limit, None);
    assert_eq!(config.heat_ring.threshold, 87);
    assert_eq!(config.heat_ring.label_template(), "{X}/{Y}");
}
