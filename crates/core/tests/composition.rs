//! Concerns that only show up when several cards are composed into one block in
//! a README: content edges lining up, and a card being no taller than it needs.

use github_personal_stats_core::{
    GithubClient, GithubStatsConfig, HeatRing, MAX_PADDING, MockGithubClient, OutputKind,
    aggregate_card_data, render_card,
};

const FIXTURE: &str = include_str!("fixtures/github_user_data.json");

fn card(kind: OutputKind, config: &GithubStatsConfig) -> String {
    let data = MockGithubClient::success(FIXTURE)
        .fetch_user_data(config)
        .unwrap();
    render_card(
        &aggregate_card_data(&data, kind, &HeatRing::default()),
        config,
    )
}

fn first_text_x(svg: &str) -> u32 {
    svg.split("<text")
        .nth(1)
        .and_then(|chunk| chunk.split_once(r#"x=""#))
        .and_then(|(_, rest)| rest.split_once('"'))
        .and_then(|(value, _)| value.parse().ok())
        .expect("the card should draw text")
}

fn stats_at(width: u32, padding: Option<&str>) -> String {
    let mut config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(width, 200)
        .unwrap();
    if let Some(padding) = padding {
        config = config.with_padding(padding).unwrap();
    }
    card(OutputKind::Stats, &config)
}

/// Default padding scales with width, which is right for a card seen on its own.
#[test]
fn padding_still_grows_with_the_card_when_it_is_left_alone() {
    assert!(first_text_x(&stats_at(275, None)) < first_text_x(&stats_at(550, None)));
}

/// Two tiles of different widths stacked in a README should line their content
/// up, which they cannot do while padding is derived from width.
#[test]
fn pinned_padding_lines_up_tiles_of_different_widths() {
    assert_eq!(
        first_text_x(&stats_at(275, Some("20"))),
        first_text_x(&stats_at(550, Some("20"))),
    );
}

#[test]
fn padding_can_be_handed_back_to_the_card() {
    assert_eq!(
        first_text_x(&stats_at(550, Some("auto"))),
        first_text_x(&stats_at(550, None)),
    );
}

fn auto_height_card(kind: OutputKind, width: u32) -> (u32, u32) {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(width, 999)
        .unwrap()
        .with_auto_height();
    let svg = card(kind, &config);

    let height = svg
        .split_once(r#"height=""#)
        .and_then(|(_, rest)| rest.split_once('"'))
        .and_then(|(value, _)| value.parse().ok())
        .expect("the card should declare a height");
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
        .expect("the card should draw text");

    (height, lowest)
}

/// Auto height has to earn its name from both sides: nothing may fall off the
/// bottom, and the card must not carry the dead space that a height guessed up
/// front leaves behind.
#[test]
fn auto_height_fits_the_content_without_clipping_it_or_padding_it_out() {
    for kind in [
        OutputKind::Stats,
        OutputKind::Languages,
        OutputKind::Streak,
        OutputKind::Heat,
        OutputKind::Metric,
    ] {
        for width in [275, 300, 420, 500, 700, 1000] {
            let (height, lowest) = auto_height_card(kind, width);

            assert!(
                lowest < height,
                "{kind:?} at {width}px drew a baseline at y={lowest} on a {height}px card"
            );
            assert!(
                height - lowest <= 60,
                "{kind:?} at {width}px left {} px below its last line",
                height - lowest
            );
        }
    }
}

/// The rank caption is wider than the ring it sits under, so placing the ring by
/// its own radius alone pushed the caption into the right margin on a tile.
#[test]
fn the_rank_caption_stays_inside_the_margin_at_every_width() {
    for width in [275, 300, 380, 440, 500, 700, 1000] {
        let config = GithubStatsConfig::new("octo")
            .unwrap()
            .with_size(width, 200)
            .unwrap();
        let svg = card(OutputKind::Stats, &config);

        let (centre, caption) = svg
            .split("<text")
            .skip(1)
            .filter(|chunk| chunk.contains(r#"text-anchor="middle""#))
            .filter_map(|chunk| {
                let x = chunk
                    .split_once(r#"x=""#)
                    .and_then(|(_, rest)| rest.split_once('"'))
                    .and_then(|(value, _)| value.parse::<u32>().ok())?;
                let body = chunk.split_once('>').map(|(_, body)| body)?;
                Some((x, body.to_owned()))
            })
            .find(|(_, body)| body.starts_with("RANK"))
            .expect("the stats card should caption its rank");

        let text = caption.split('<').next().unwrap_or_default();
        let half = (text.chars().count() as u32 * 6) / 2;
        let margin = width - (width / 20).clamp(16, 28);

        assert!(
            centre + half <= margin,
            "at {width}px the rank caption reaches {} past a margin at {margin}",
            centre + half
        );
    }
}

#[test]
fn auto_height_ignores_the_height_it_was_handed() {
    let (tall, _) = auto_height_card(OutputKind::Metric, 275);
    assert!(
        tall < 999,
        "a fitted card should not keep the height it was given"
    );
}

#[test]
fn a_padding_that_would_leave_no_room_for_content_is_refused() {
    let config = GithubStatsConfig::new("octo").unwrap();

    assert!(config.clone().with_padding("nonsense").is_err());
    assert!(
        config
            .clone()
            .with_padding(&(MAX_PADDING + 1).to_string())
            .is_err()
    );
    assert!(config.with_padding(&MAX_PADDING.to_string()).is_ok());
}
