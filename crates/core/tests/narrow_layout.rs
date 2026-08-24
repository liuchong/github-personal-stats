use github_personal_stats_core::{
    CardData, GithubClient, GithubStatsConfig, HeatRing, MockGithubClient, OutputKind,
    aggregate_card_data, render_card,
};

const FIXTURE: &str = include_str!("fixtures/github_user_data.json");

/// A phone-sized README column. Tiles this wide are meant to be composed side by
/// side on a desktop and to reflow into a single column on a phone.
const TILE: (u32, u32) = (275, 250);

fn streak_card() -> CardData {
    let config = GithubStatsConfig::new("octo").unwrap();
    let data = MockGithubClient::success(FIXTURE)
        .fetch_user_data(&config)
        .unwrap();
    aggregate_card_data(&data, OutputKind::Streak, &HeatRing::default())
}

/// Baseline of the first `<text>` whose body contains `needle`, plus the `x` it
/// was drawn at.
fn text_at(svg: &str, needle: &str) -> (u32, u32) {
    svg.split("<text")
        .find(|chunk| {
            chunk
                .split_once('>')
                .is_some_and(|(_, body)| body.starts_with(needle))
        })
        .map(|chunk| (attr(chunk, "x"), attr(chunk, "y")))
        .unwrap_or_else(|| panic!("no text starting with {needle:?}"))
}

/// Baseline of the date line the ring draws under its own caption, which is the
/// lowest thing the ring itself owns.
fn ring_caption_bottom(svg: &str) -> u32 {
    let (_, caption) = text_at(svg, "Current Streak");
    caption + 17
}

fn attr(chunk: &str, name: &str) -> u32 {
    let key = format!("{name}=\"");
    chunk
        .split_once(key.as_str())
        .and_then(|(_, rest)| rest.split_once('"'))
        .and_then(|(value, _)| value.parse().ok())
        .unwrap_or_else(|| panic!("no numeric {name} attribute"))
}

#[test]
fn a_phone_width_streak_card_drops_its_figures_below_the_ring_instead_of_beside_it() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(TILE.0, TILE.1)
        .unwrap();
    let svg = render_card(&streak_card(), &config);

    let ring_dates = ring_caption_bottom(&svg);
    let (total_x, total_y) = text_at(&svg, "Total Contributions");
    let (longest_x, longest_y) = text_at(&svg, "Longest Streak");

    // Both figures clear the ring's own date line rather than sharing its row.
    assert!(
        total_y > ring_dates,
        "figures should sit below the ring caption, got {total_y} vs {ring_dates}"
    );
    assert_eq!(
        total_y, longest_y,
        "the two figures should share one row beneath the ring"
    );
    assert!(
        longest_x > total_x,
        "the two figures should still read left to right"
    );
}

#[test]
fn a_wide_streak_card_keeps_the_three_columns_it_has_always_drawn() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(1000, 220)
        .unwrap();
    let svg = render_card(&streak_card(), &config);

    let ring_dates = ring_caption_bottom(&svg);
    let (_, total_y) = text_at(&svg, "Total Contributions");

    assert!(
        total_y < ring_dates,
        "a wide card should keep its figures level with the ring, not under it"
    );
}

#[test]
fn a_phone_width_streak_card_keeps_every_line_inside_the_canvas() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(TILE.0, TILE.1)
        .unwrap();
    let svg = render_card(&streak_card(), &config);

    let lowest = svg
        .split("<text")
        .skip(1)
        .map(|chunk| attr(chunk, "y"))
        .max()
        .expect("the card should draw text");

    assert!(
        lowest < TILE.1,
        "text at y={lowest} would fall outside a {}px tall tile",
        TILE.1
    );
}
