use crate::{
    AggregatedStats, CardData, CodingActivitySummary, GithubStatsConfig, HEAT_RAMP_STEPS, HeatRing,
    HeatScale, HeatShape, HeatWindow, ImageSize, LanguageShare, StatMetric, StreakMetric,
    StreakSummary, Theme, TileMetric,
};

const FONT_STACK: &str =
    "-apple-system, BlinkMacSystemFont, 'Segoe UI', Inter, 'Helvetica Neue', Arial, sans-serif";

const MINIMUM_BAND_WIDTH: f64 = 4.0;

const NARROW_WIDTH: u32 = 440;

fn is_narrow(width: u32) -> bool {
    width < NARROW_WIDTH
}

/// `role="img"` needs an accessible name or assistive technology announces the
/// card as an unlabelled image, so every card names itself.
const TITLE_ID: &str = "gps-title";

const GUTTER: u32 = 24;

/// Where the ring's two caption lines sit below its edge. Anything stacked under
/// a ring measures from these, so the block cannot drift into the caption.
const RING_CAPTION_OFFSET: u32 = 26;
const RING_DATE_OFFSET: u32 = 43;

/// Geometry of the stacked streak layout, and the height it therefore needs.
/// Stacking only pays off when the card is tall enough to hold the ring, its two
/// caption lines, and a row of figures; on a short card the three columns still
/// read better than a stack that would run off the bottom edge.
const STACKED_RING_TOP: u32 = 30;
const STACKED_RING_RADIUS: u32 = 26;
const STACKED_FIGURE_GAP: u32 = 28;
const FIGURE_LABEL_TO_NOTE: u32 = 50;
const STACKED_STREAK_HEIGHT: u32 = STACKED_RING_TOP
    + STACKED_RING_RADIUS * 2
    + RING_DATE_OFFSET
    + STACKED_FIGURE_GAP
    + FIGURE_LABEL_TO_NOTE
    + DESCENDER;

/// Slack left under the lowest baseline so descenders are not clipped.
const DESCENDER: u32 = 8;

/// Baseline spacing inside a single-figure tile. A tile centres its block, so the
/// natural height is what leaves the note exactly one descender clear of the
/// bottom edge: (h - block) / 2 + 12 + block + descender = h.
const METRIC_LABEL_TO_VALUE: u32 = 38;
const METRIC_LABEL_TO_NOTE: u32 = 60;
const METRIC_BLOCK_HEIGHT: u32 = METRIC_LABEL_TO_NOTE + 40;

/// Rows of the stats panel: where the first baseline sits and the spacing the
/// rows reach when the card is not cramped.
const STAT_ROWS_TOP: u32 = 58;
const STAT_ROW_STEP: u32 = 30;

/// Rows of the languages panel, as tracks on a narrow card and as two columns on
/// a wide one.
const LANGUAGE_TRACK_TOP: u32 = 56;
const LANGUAGE_TRACK_STEP: u32 = 20;
const LANGUAGE_COLUMN_TOP: u32 = 72;
const LANGUAGE_COLUMN_STEP: u32 = 24;

/// The three-column streak layout, measured to its lowest baseline. That is the
/// ring's date line rather than the side notes, because the ring hangs lower than
/// the figures beside it.
const COLUMN_RING_CENTRE: u32 = 70;
const COLUMN_RING_RADIUS: u32 = 32;
const COLUMN_STREAK_HEIGHT: u32 =
    COLUMN_RING_CENTRE + COLUMN_RING_RADIUS + RING_DATE_OFFSET + DESCENDER;

/// A centred ring needs its two caption lines plus balanced margins.
const RING_BLOCK_SLACK: u32 = RING_DATE_OFFSET + DESCENDER * 2 + 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTheme {
    pub kind: Theme,
    pub background: &'static str,
    pub ink: &'static str,
    pub muted: &'static str,
    pub line: &'static str,
    pub track: &'static str,
    pub accent: &'static str,
    pub on_accent: &'static str,
}

impl RenderTheme {
    pub fn new(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self {
                kind: theme,
                background: "#0d1117",
                ink: "#e6edf3",
                muted: "#8b949e",
                line: "#21262d",
                track: "#262c36",
                accent: "#4493f8",
                on_accent: "#0d1117",
            },
            Theme::Transparent => Self {
                kind: theme,
                background: "transparent",
                ink: "#1f2328",
                muted: "#59636e",
                line: "#d8dee6",
                track: "#dde3ea",
                accent: "#0969da",
                on_accent: "#ffffff",
            },
            Theme::Light => Self {
                kind: theme,
                background: "#ffffff",
                ink: "#15181d",
                muted: "#656d76",
                line: "#ebedf0",
                track: "#e4e8ee",
                accent: "#0b69d4",
                on_accent: "#ffffff",
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Rect {
    fn right(self) -> u32 {
        self.x + self.width
    }

    fn is_narrow(self) -> bool {
        is_narrow(self.width)
    }
}

pub fn render_card(card: &CardData, config: &GithubStatsConfig) -> String {
    if config.auto_height {
        let mut resolved = config.clone();
        resolved.size.height = natural_height(card, config);
        resolved.auto_height = false;
        return render_card(card, &resolved);
    }

    let theme = RenderTheme::new(config.theme);
    let title = card_title(card, config);
    match card {
        CardData::Dashboard {
            stats,
            languages,
            streak,
        } => render_dashboard(stats, languages, streak, config, &theme, &title),
        CardData::Stats(stats) => render_stats_card(stats, config, &theme, &title),
        CardData::Languages(languages) => render_languages_card(languages, config, &theme, &title),
        CardData::Streak(streak) => render_streak_card(streak, config, &theme, &title),
        CardData::Heat(streak) => render_heat_card(streak, config, &theme, &title),
        CardData::Metric { stats, streak } => {
            render_metric_card(stats, streak, config, &theme, &title)
        }
        CardData::Activity(summary) => render_activity_card(summary, config, &theme, &title),
        CardData::Status { state } => render_status_card(state, config, &theme, &title),
    }
}

fn card_title(card: &CardData, config: &GithubStatsConfig) -> String {
    let username = &config.username;
    match card {
        CardData::Dashboard { .. } => format!("GitHub profile summary for {username}"),
        CardData::Stats(_) => format!("GitHub stats for {username}"),
        CardData::Languages(_) => format!("Top languages for {username}"),
        CardData::Streak(_) => format!("Contribution streak for {username}"),
        CardData::Heat(_) => format!("Contribution heat ring for {username}"),
        CardData::Metric { .. } => format!("{} for {username}", metric_label(config.metric)),
        CardData::Activity(_) => format!("Coding activity for {username}"),
        CardData::Status { state } => format!("Service status: {state}"),
    }
}

pub fn render_readme_section(summary: &CodingActivitySummary, title: &str) -> String {
    let mut lines = vec![format!("### {}", escape_markdown(title)), String::new()];

    for entry in &summary.entries {
        lines.push(format!(
            "{} {}",
            entry.language,
            progress_bar(entry.seconds, summary.total_seconds)
        ));
    }

    if let Some(masked) = summary.masked_total_seconds {
        lines.push(format!("Total: {}", format_duration(masked)));
    } else {
        lines.push(format!("Total: {}", format_duration(summary.total_seconds)));
    }

    lines.join("\n")
}

fn render_dashboard(
    stats: &AggregatedStats,
    languages: &[LanguageShare],
    streak: &StreakSummary,
    config: &GithubStatsConfig,
    theme: &RenderTheme,
    title: &str,
) -> String {
    let size = &config.size;
    let pad = padding(config);
    let split = size.height * 53 / 100;
    let column = size.width.saturating_sub(pad * 2 + GUTTER) / 2;
    let top_height = split.saturating_sub(pad + 16);
    let stats_area = Rect {
        x: pad,
        y: pad,
        width: column,
        height: top_height,
    };
    let languages_area = Rect {
        x: pad + column + GUTTER,
        y: pad,
        width: column,
        height: top_height,
    };
    let streak_area = Rect {
        x: pad,
        y: split + 20,
        width: size.width.saturating_sub(pad * 2),
        height: size.height.saturating_sub(split + 20 + pad),
    };

    svg_root(
        size,
        config.scale_basis_points,
        theme,
        title,
        format!(
            "{}{}{}{}{}",
            stats_section(stats_area, stats, theme, &config.stat_rows),
            vertical_rule(pad + column + GUTTER / 2, pad + 2, pad + top_height, theme),
            languages_section(languages_area, languages, theme, config.language_rows),
            horizontal_rule(pad, size.width - pad, split, theme),
            streak_section(
                streak_area,
                streak,
                theme,
                &config.heat_ring,
                config.streak_sides,
            ),
        ),
    )
}

fn render_stats_card(
    stats: &AggregatedStats,
    config: &GithubStatsConfig,
    theme: &RenderTheme,
    title: &str,
) -> String {
    let size = &config.size;
    svg_root(
        size,
        config.scale_basis_points,
        theme,
        title,
        stats_section(
            card_area(size, padding(config)),
            stats,
            theme,
            &config.stat_rows,
        ),
    )
}

fn render_languages_card(
    languages: &[LanguageShare],
    config: &GithubStatsConfig,
    theme: &RenderTheme,
    title: &str,
) -> String {
    let size = &config.size;
    svg_root(
        size,
        config.scale_basis_points,
        theme,
        title,
        languages_section(
            card_area(size, padding(config)),
            languages,
            theme,
            config.language_rows,
        ),
    )
}

fn render_streak_card(
    streak: &StreakSummary,
    config: &GithubStatsConfig,
    theme: &RenderTheme,
    title: &str,
) -> String {
    let size = &config.size;
    svg_root(
        size,
        config.scale_basis_points,
        theme,
        title,
        streak_section(
            card_area(size, padding(config)),
            streak,
            theme,
            &config.heat_ring,
            config.streak_sides,
        ),
    )
}

fn render_activity_card(
    summary: &CodingActivitySummary,
    config: &GithubStatsConfig,
    theme: &RenderTheme,
    title: &str,
) -> String {
    let size = &config.size;
    let pad = padding(config);
    svg_root(
        size,
        config.scale_basis_points,
        theme,
        title,
        activity_section(card_area(size, pad), summary, theme),
    )
}

fn render_status_card(
    state: &str,
    config: &GithubStatsConfig,
    theme: &RenderTheme,
    title: &str,
) -> String {
    let size = &config.size;
    let area = card_area(size, padding(config));
    svg_root(
        size,
        config.scale_basis_points,
        theme,
        title,
        format!(
            "{}{}{}",
            eyebrow(area.x, area.y + 16, "Status", theme),
            badge(area.x, area.y + 44, state, theme),
            text(area.x, area.y + 96, 11.0, theme.muted, "Service health"),
        ),
    )
}

/// The height at which a card's content sits at its natural spacing: tall enough
/// that nothing is cramped or clipped, and no taller. Every figure here is
/// derived from the constant the matching layout measures with, so the two
/// cannot drift apart.
///
/// Cards that divide a given height between sections have no natural height of
/// their own and keep the height they were asked for; the command line refuses
/// `auto` for those rather than quietly ignoring it.
fn natural_height(card: &CardData, config: &GithubStatsConfig) -> u32 {
    let pad = padding(config);
    let width = config.size.width.saturating_sub(pad * 2);
    let content = match card {
        CardData::Stats(_) => STAT_ROWS_TOP + STAT_ROW_STEP * config.stat_rows.len().max(1) as u32,
        CardData::Languages(languages) => language_natural_height(width, languages, config),
        CardData::Streak(_) => {
            if is_narrow(width) {
                STACKED_STREAK_HEIGHT
            } else {
                COLUMN_STREAK_HEIGHT
            }
        }
        CardData::Heat(_) => ring_radius_for(width) * 2 + RING_BLOCK_SLACK,
        CardData::Metric { .. } => METRIC_BLOCK_HEIGHT,
        _ => return config.size.height,
    };

    content + pad * 2
}

fn language_natural_height(
    width: u32,
    languages: &[LanguageShare],
    config: &GithubStatsConfig,
) -> u32 {
    let drawn = languages.len().min(config.language_rows).max(1) as u32;
    if is_narrow(width) {
        LANGUAGE_TRACK_TOP + LANGUAGE_TRACK_STEP * drawn
    } else {
        let per_column = config.language_rows.div_ceil(2).max(1) as u32;
        let rows = drawn.min(per_column);
        LANGUAGE_COLUMN_TOP + LANGUAGE_COLUMN_STEP * (rows.saturating_sub(1)) + DESCENDER + 4
    }
}

fn ring_radius_for(width: u32) -> u32 {
    if width >= 180 { 32 } else { 26 }
}

fn card_area(size: &ImageSize, pad: u32) -> Rect {
    Rect {
        x: pad,
        y: pad,
        width: size.width.saturating_sub(pad * 2),
        height: size.height.saturating_sub(pad * 2),
    }
}

/// Padding scales with width by default, which suits a card seen on its own but
/// misaligns tiles of different widths composed into one block, so it can also
/// be pinned outright.
fn padding(config: &GithubStatsConfig) -> u32 {
    config
        .padding
        .unwrap_or_else(|| (config.size.width / 20).clamp(16, 28))
}

/// The card is laid out in viewBox units and displayed at whatever the scale
/// multiplies those to. Because the drawing is vector, a scaled card is not
/// resampled: the same geometry simply arrives larger or smaller.
fn svg_root(
    size: &ImageSize,
    scale_basis_points: u32,
    theme: &RenderTheme,
    title: &str,
    body: String,
) -> String {
    let scaled = |value: u32| {
        (u64::from(value) * u64::from(scale_basis_points) / 10_000)
            .try_into()
            .unwrap_or(u32::MAX)
    };

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{display_width}" height="{display_height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="{TITLE_ID}" shape-rendering="geometricPrecision" text-rendering="optimizeLegibility" font-family="{font}" style="font-variant-numeric:tabular-nums"><title id="{TITLE_ID}">{title}</title><rect width="100%" height="100%" fill="{background}"/>{body}</svg>"#,
        display_width = scaled(size.width),
        display_height = scaled(size.height),
        width = size.width,
        height = size.height,
        font = FONT_STACK,
        title = escape_xml(title),
        background = theme.background,
        body = body,
    )
}

fn stats_section(
    area: Rect,
    stats: &AggregatedStats,
    theme: &RenderTheme,
    metrics: &[StatMetric],
) -> String {
    let radius = (area.width / 12).clamp(20, 38);
    let ring_cx = area.right().saturating_sub(rank_block_inset(stats, radius));
    let ring_cy = area.y + 34 + area.height.saturating_sub(34) / 2 - 10;
    let value_x = ring_cx.saturating_sub(radius + 30);
    let step = (area.height.saturating_sub(STAT_ROWS_TOP) / metrics.len().max(1) as u32)
        .clamp(20, STAT_ROW_STEP);

    let rows = metrics
        .iter()
        .enumerate()
        .map(|(index, metric)| {
            let (label, value, icon_kind) = stat_metric(*metric, stats);
            stat_row(
                area.x,
                area.y + STAT_ROWS_TOP - 4 + index as u32 * step,
                value_x,
                label,
                value,
                icon_kind,
                theme,
            )
        })
        .collect::<String>();

    format!(
        "{}{}{}",
        eyebrow(area.x, area.y + 16, "Stats", theme),
        rows,
        rank_ring(ring_cx, ring_cy, radius, stats, theme),
    )
}

fn stat_metric(metric: StatMetric, stats: &AggregatedStats) -> (&'static str, u64, IconKind) {
    let (value, icon) = match metric {
        StatMetric::Stars => (stats.total_stars, IconKind::Star),
        StatMetric::Commits => (stats.total_commits, IconKind::Commit),
        StatMetric::PullRequests => (stats.total_pull_requests, IconKind::PullRequest),
        StatMetric::Issues => (stats.total_issues, IconKind::Issue),
        StatMetric::Reviews => (stats.total_reviews, IconKind::Review),
        StatMetric::ContributedTo => (stats.contributed_to, IconKind::Repository),
    };

    (stat_label(metric), value, icon)
}

fn stat_label(metric: StatMetric) -> &'static str {
    match metric {
        StatMetric::Stars => "Total Stars",
        StatMetric::Commits => "Commits",
        StatMetric::PullRequests => "Pull Requests",
        StatMetric::Issues => "Issues",
        StatMetric::Reviews => "Reviews",
        StatMetric::ContributedTo => "Contributed To",
    }
}

fn streak_label(metric: StreakMetric) -> &'static str {
    match metric {
        StreakMetric::TotalContributions => "Total Contributions",
        StreakMetric::LongestStreak => "Longest Streak",
        StreakMetric::CurrentStreak => "Current Streak",
        StreakMetric::ActiveDays => "Active Days",
    }
}

fn metric_label(metric: TileMetric) -> &'static str {
    match metric {
        TileMetric::Stat(metric) => stat_label(metric),
        TileMetric::Streak(metric) => streak_label(metric),
    }
}

fn stat_row(
    x: u32,
    y: u32,
    value_x: u32,
    label: &str,
    value: u64,
    icon_kind: IconKind,
    theme: &RenderTheme,
) -> String {
    format!(
        "{}{}{}",
        icon(icon_kind, x, y - 11, 14, theme.muted),
        text(x + 22, y, 12.5, theme.ink, label),
        text_end(value_x, y, 12.5, theme.ink, &format_number(value)),
    )
}

/// How far the ring's centre sits from the right edge. The caption under the ring
/// is wider than the ring itself, so on a narrow card the ring has to come in far
/// enough for the caption to clear the margin, not just the ring.
fn rank_block_inset(stats: &AggregatedStats, radius: u32) -> u32 {
    let caption_half = advance(&rank_caption(stats), RANK_CAPTION_SIZE) / 2;

    (radius + 10).max(caption_half + 4)
}

fn rank_caption(stats: &AggregatedStats) -> String {
    format!("RANK · {}", format_number(stats.score))
}

const RANK_CAPTION_SIZE: f32 = 10.0;

fn rank_ring(
    cx: u32,
    cy: u32,
    radius: u32,
    stats: &AggregatedStats,
    theme: &RenderTheme,
) -> String {
    let circumference = 2.0 * std::f64::consts::PI * f64::from(radius);
    let closure = f64::from(10_000 - stats.percentile_basis_points.min(10_000)) / 10_000.0;
    let letter_size = (radius * 4 / 5).max(16);

    format!(
        concat!(
            r#"<circle cx="{cx}" cy="{cy}" r="{radius}" fill="none" stroke="{track}" stroke-width="3"/>"#,
            r#"<circle cx="{cx}" cy="{cy}" r="{radius}" fill="none" stroke="{accent}" stroke-width="3" stroke-linecap="round" stroke-dasharray="{filled:.1} {circumference:.1}" transform="rotate(-90 {cx} {cy})"/>"#,
            "{letter}{caption}",
        ),
        cx = cx,
        cy = cy,
        radius = radius,
        track = theme.track,
        accent = theme.accent,
        filled = circumference * closure,
        circumference = circumference,
        letter = text_middle(
            cx,
            cy + letter_size / 3,
            letter_size as f32,
            600,
            theme.ink,
            stats.rank,
        ),
        caption = text_middle(
            cx,
            cy + radius + 18,
            RANK_CAPTION_SIZE,
            400,
            theme.muted,
            &rank_caption(stats),
        ),
    )
}

fn languages_section(
    area: Rect,
    languages: &[LanguageShare],
    theme: &RenderTheme,
    rows_wanted: usize,
) -> String {
    let rows = if area.is_narrow() {
        language_track_rows(area, languages, theme, rows_wanted)
    } else {
        language_columns(area, languages, theme, rows_wanted)
    };

    format!(
        "{}{}{}",
        eyebrow(area.x, area.y + 16, "Languages", theme),
        stacked_language_bar(
            area.x,
            area.y + 32,
            area.width,
            languages,
            theme,
            rows_wanted
        ),
        rows,
    )
}

fn language_columns(
    area: Rect,
    languages: &[LanguageShare],
    theme: &RenderTheme,
    rows_wanted: usize,
) -> String {
    let per_column = rows_wanted.div_ceil(2).max(1);
    let column = area.width.saturating_sub(GUTTER) / 2;

    languages
        .iter()
        .take(rows_wanted)
        .enumerate()
        .map(|(index, language)| {
            let x = area.x + (index / per_column) as u32 * (column + GUTTER);
            let y =
                area.y + LANGUAGE_COLUMN_TOP + (index % per_column) as u32 * LANGUAGE_COLUMN_STEP;
            format!(
                "{}{}{}",
                language_dot(x + 4, y - 4, 4.0, language, index),
                text(x + 16, y, 12.0, theme.ink, &language.name),
                text_end(x + column, y, 12.0, theme.muted, &share(language)),
            )
        })
        .collect()
}

fn language_track_rows(
    area: Rect,
    languages: &[LanguageShare],
    theme: &RenderTheme,
    rows_wanted: usize,
) -> String {
    let name_column = (area.width * 28 / 100).clamp(90, 150);
    let track_x = area.x + name_column;
    let track_width = area
        .width
        .saturating_sub(name_column + 52)
        .max(TRACK_MINIMUM);
    let step = (area.height.saturating_sub(LANGUAGE_TRACK_TOP) / rows_wanted.max(1) as u32)
        .clamp(15, LANGUAGE_TRACK_STEP);

    languages
        .iter()
        .take(rows_wanted)
        .enumerate()
        .map(|(index, language)| {
            let y = area.y + LANGUAGE_TRACK_TOP + index as u32 * step;
            let filled = track_width * language.percentage_basis_points / 10_000;
            format!(
                "{}{}{}{}{}",
                language_dot(area.x + 4, y - 4, 3.5, language, index),
                text(area.x + 15, y, 11.5, theme.ink, &language.name),
                rounded_rect(track_x, y - 7, track_width, 4, theme.track),
                rounded_rect(
                    track_x,
                    y - 7,
                    filled,
                    4,
                    language_color(&language.name, index)
                ),
                text_end(area.right(), y, 11.5, theme.muted, &share(language)),
            )
        })
        .collect()
}

const TRACK_MINIMUM: u32 = 24;

fn stacked_language_bar(
    x: u32,
    y: u32,
    width: u32,
    languages: &[LanguageShare],
    theme: &RenderTheme,
    rows_wanted: usize,
) -> String {
    let mut consumed_basis_points = 0;
    let mut previous_edge = 0;
    let mut segments = String::new();

    for (index, language) in languages.iter().take(rows_wanted).enumerate() {
        consumed_basis_points += language.percentage_basis_points;
        let edge = width * consumed_basis_points.min(10_000) / 10_000;
        segments.push_str(&format!(
            r#"<rect x="{}" y="{}" width="{}" height="5" fill="{}"/>"#,
            x + previous_edge,
            y,
            edge.saturating_sub(previous_edge),
            language_color(&language.name, index),
        ));
        previous_edge = edge;
    }

    format!(
        concat!(
            r#"<defs><clipPath id="gps-language-bar"><rect x="{x}" y="{y}" width="{width}" height="5" rx="2.5"/></clipPath></defs>"#,
            r#"<rect x="{x}" y="{y}" width="{width}" height="5" rx="2.5" fill="{track}"/>"#,
            r#"<g clip-path="url(#gps-language-bar)">{segments}</g>"#,
        ),
        x = x,
        y = y,
        width = width,
        track = theme.track,
        segments = segments,
    )
}

fn streak_section(
    area: Rect,
    streak: &StreakSummary,
    theme: &RenderTheme,
    config: &HeatRing,
    sides: [StreakMetric; 2],
) -> String {
    if area.is_narrow() && area.height >= STACKED_STREAK_HEIGHT {
        return streak_stacked(area, streak, theme, config, sides);
    }

    streak_columns(area, streak, theme, config, sides)
}

/// The ring on its own, centred in whatever tile it is given. It carries its own
/// caption, so it needs no section eyebrow above it.
fn render_heat_card(
    streak: &StreakSummary,
    config: &GithubStatsConfig,
    theme: &RenderTheme,
    title: &str,
) -> String {
    let size = &config.size;
    let area = card_area(size, padding(config));
    let radius = ring_radius_for(area.width);
    let block = radius * 2 + RING_DATE_OFFSET + 4;
    let cy = area.y + area.height.saturating_sub(block) / 2 + radius;

    svg_root(
        size,
        config.scale_basis_points,
        theme,
        title,
        current_streak_ring(
            &Ring {
                cx: area.x + area.width / 2,
                cy,
                radius,
                compact: area.is_narrow(),
                theme,
                config: &config.heat_ring,
                stops: config.heat_ring.ramp.stops(theme.kind),
            },
            streak,
        ),
    )
}

/// A single figure, centred. Lets a README place one number wherever it likes
/// instead of taking the whole panel it normally lives in.
fn render_metric_card(
    stats: &AggregatedStats,
    streak: &StreakSummary,
    config: &GithubStatsConfig,
    theme: &RenderTheme,
    title: &str,
) -> String {
    let size = &config.size;
    let area = card_area(size, padding(config));
    let value_size = if area.width >= 200 { 34.0 } else { 26.0 };
    // The date note is supplementary, so a tile too short for all three lines
    // drops it rather than drawing it past the bottom edge.
    let room_for_note = area.height >= METRIC_LABEL_TO_NOTE + DESCENDER * 2;
    let block = if room_for_note {
        METRIC_LABEL_TO_NOTE
    } else {
        METRIC_LABEL_TO_VALUE
    };
    let label_y = area.y + area.height.saturating_sub(block) / 2 + 12;
    let placement = SidePlacement {
        label_y,
        value_y: label_y + METRIC_LABEL_TO_VALUE,
        note_y: label_y + METRIC_LABEL_TO_NOTE,
        value_size,
        compact: area.is_narrow(),
        align: Align::Centre,
    };
    let centre = area.x + area.width / 2;

    let mut figure = match config.metric {
        TileMetric::Streak(metric) => streak_side(metric, centre, placement, streak, theme),
        TileMetric::Stat(metric) => {
            let (label, value, _) = stat_metric(metric, stats);
            SideMetric {
                x: centre,
                label_y: placement.label_y,
                value_y: placement.value_y,
                note_y: placement.note_y,
                value_size: placement.value_size,
                label,
                value: format_number(value),
                unit: "",
                note: String::new(),
                theme,
                align: placement.align,
            }
        }
    };

    if !room_for_note {
        figure.note = String::new();
    }

    svg_root(
        size,
        config.scale_basis_points,
        theme,
        title,
        side_metric(figure),
    )
}

/// Three columns need room the ring's own date line already struggles for, so a
/// narrow card gives the ring a full-width row of its own and sits the two
/// figures side by side underneath it.
fn streak_stacked(
    area: Rect,
    streak: &StreakSummary,
    theme: &RenderTheme,
    config: &HeatRing,
    sides: [StreakMetric; 2],
) -> String {
    let ring_radius = STACKED_RING_RADIUS;
    let ring_cx = area.x + area.width / 2;
    let ring_cy = area.y + STACKED_RING_TOP + ring_radius;
    let ring_bottom = ring_cy + ring_radius + RING_DATE_OFFSET;
    let figures_y = ring_bottom + STACKED_FIGURE_GAP;
    let column = area.width / 2;
    let placement = SidePlacement {
        label_y: figures_y,
        value_y: figures_y + 30,
        note_y: figures_y + FIGURE_LABEL_TO_NOTE,
        value_size: 22.0,
        compact: true,
        align: Align::Centre,
    };

    let figures = sides
        .iter()
        .enumerate()
        .map(|(index, metric)| {
            side_metric(streak_side(
                *metric,
                area.x + index as u32 * column + column / 2,
                SidePlacement { ..placement },
                streak,
                theme,
            ))
        })
        .collect::<String>();

    format!(
        "{}{}{}{}",
        eyebrow(area.x, area.y + 12, "Streak", theme),
        current_streak_ring(
            &Ring {
                cx: ring_cx,
                cy: ring_cy,
                radius: ring_radius,
                compact: true,
                theme,
                config,
                stops: config.ramp.stops(theme.kind),
            },
            streak
        ),
        horizontal_rule(area.x, area.right(), ring_bottom + 12, theme),
        figures,
    )
}

fn streak_columns(
    area: Rect,
    streak: &StreakSummary,
    theme: &RenderTheme,
    config: &HeatRing,
    sides: [StreakMetric; 2],
) -> String {
    let column = area.width / 3;
    // A card too short to stack still has to fit three columns, so the figures
    // and the ring tighten rather than run past the bottom edge.
    let compact = area.is_narrow();
    let (label_y, value_y, note_y) = if compact {
        (area.y + 44, area.y + 80, area.y + 104)
    } else {
        (area.y + 52, area.y + 92, area.y + 118)
    };
    let value_size = if compact { 26.0 } else { 34.0 };
    let ring_radius = if compact { 26 } else { COLUMN_RING_RADIUS };
    let ring_cy = area.y + if compact { 62 } else { COLUMN_RING_CENTRE };
    let ring_cx = area.x + column + column / 2;

    let total = side_metric(streak_side(
        sides[0],
        area.x,
        SidePlacement {
            label_y,
            value_y,
            note_y,
            value_size,
            compact,
            align: Align::Left,
        },
        streak,
        theme,
    ));
    let longest = side_metric(streak_side(
        sides[1],
        area.x + column * 2,
        SidePlacement {
            label_y,
            value_y,
            note_y,
            value_size,
            compact,
            align: Align::Left,
        },
        streak,
        theme,
    ));

    format!(
        "{}{}{}{}{}{}",
        eyebrow(area.x, area.y + 12, "Streak", theme),
        total,
        vertical_rule(
            area.x + column - 12,
            area.y + 20,
            area.y + area.height,
            theme
        ),
        current_streak_ring(
            &Ring {
                cx: ring_cx,
                cy: ring_cy,
                radius: ring_radius,
                compact,
                theme,
                config,
                stops: config.ramp.stops(theme.kind),
            },
            streak
        ),
        vertical_rule(
            area.x + column * 2 - 12,
            area.y + 20,
            area.y + area.height,
            theme
        ),
        longest,
    )
}

struct SidePlacement {
    label_y: u32,
    value_y: u32,
    note_y: u32,
    value_size: f32,
    compact: bool,
    align: Align,
}

/// Each panel reports one figure with the date range that figure actually covers,
/// so the note never describes a span other than the number above it.
/// The value and its unit are one visual word, so they are centred as a group:
/// centring the number alone would push the unit off balance.
fn centred_metric(metric: SideMetric<'_>) -> String {
    let label_size = if metric.value_size > 30.0 { 11.0 } else { 10.0 };
    let value_advance = advance(&metric.value, metric.value_size);
    let unit_advance = if metric.unit.is_empty() {
        0
    } else {
        advance(metric.unit, 11.5) + 6
    };
    let start = metric.x.saturating_sub((value_advance + unit_advance) / 2);

    let unit = if metric.unit.is_empty() {
        String::new()
    } else {
        text(
            start + value_advance + 6,
            metric.value_y,
            11.5,
            metric.theme.muted,
            metric.unit,
        )
    };

    format!(
        "{}{}{}{}",
        text_middle(
            metric.x,
            metric.label_y,
            label_size,
            400,
            metric.theme.muted,
            metric.label
        ),
        text_weighted(
            start,
            metric.value_y,
            metric.value_size,
            600,
            metric.theme.ink,
            &metric.value
        ),
        unit,
        if metric.note.is_empty() {
            String::new()
        } else {
            text_middle(
                metric.x,
                metric.note_y,
                label_size,
                400,
                metric.theme.muted,
                &metric.note,
            )
        },
    )
}

/// Rough width of a run of text at a given size. The card fonts are proportional
/// so this only needs to be close enough to centre a short figure.
fn advance(value: &str, size: f32) -> u32 {
    (value.chars().count() as f32 * size * 0.6) as u32
}

fn streak_side<'a>(
    metric: StreakMetric,
    x: u32,
    placement: SidePlacement,
    streak: &StreakSummary,
    theme: &'a RenderTheme,
) -> SideMetric<'a> {
    let (label, value, unit, note) = match metric {
        StreakMetric::TotalContributions => (
            streak_label(metric),
            format_number(streak.total_contributions),
            "",
            streak
                .current_end
                .as_deref()
                .map(|date| format!("through {}", format_single_date(date)))
                .unwrap_or_default(),
        ),
        StreakMetric::LongestStreak => (
            streak_label(metric),
            streak.longest.to_string(),
            "days",
            date_range(
                &streak.longest_start,
                &streak.longest_end,
                placement.compact,
            ),
        ),
        StreakMetric::CurrentStreak => (
            streak_label(metric),
            streak.current.to_string(),
            "days",
            date_range(
                &streak.current_start,
                &streak.current_end,
                placement.compact,
            ),
        ),
        StreakMetric::ActiveDays => (
            streak_label(metric),
            format_number(u64::from(streak.total_active_days)),
            "",
            streak
                .current_end
                .as_deref()
                .map(|date| format!("through {}", format_single_date(date)))
                .unwrap_or_default(),
        ),
    };

    SideMetric {
        x,
        label_y: placement.label_y,
        value_y: placement.value_y,
        note_y: placement.note_y,
        value_size: placement.value_size,
        label,
        value,
        unit,
        note,
        theme,
        align: placement.align,
    }
}

struct SideMetric<'a> {
    x: u32,
    label_y: u32,
    value_y: u32,
    note_y: u32,
    value_size: f32,
    label: &'a str,
    value: String,
    unit: &'a str,
    note: String,
    theme: &'a RenderTheme,
    align: Align,
}

/// Whether a figure reads from a left edge or about a centre line. Columns of
/// figures align left; a figure that owns its width sits centred, so it agrees
/// with the ring above it instead of drifting to one side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Centre,
}

fn side_metric(metric: SideMetric<'_>) -> String {
    if metric.align == Align::Centre {
        return centred_metric(metric);
    }

    let label_size = if metric.value_size > 30.0 { 11.0 } else { 10.0 };
    let unit = if metric.unit.is_empty() {
        String::new()
    } else {
        text(
            metric.x + advance(&metric.value, metric.value_size) + 6,
            metric.value_y,
            11.5,
            metric.theme.muted,
            metric.unit,
        )
    };

    format!(
        "{}{}{}{}",
        text(
            metric.x,
            metric.label_y,
            label_size,
            metric.theme.muted,
            metric.label
        ),
        text_weighted(
            metric.x,
            metric.value_y,
            metric.value_size,
            600,
            metric.theme.ink,
            &metric.value
        ),
        unit,
        text(
            metric.x,
            metric.note_y,
            label_size,
            metric.theme.muted,
            &metric.note
        ),
    )
}

struct Ring<'a> {
    cx: u32,
    cy: u32,
    radius: u32,
    compact: bool,
    theme: &'a RenderTheme,
    config: &'a HeatRing,
    stops: Vec<String>,
}

fn current_streak_ring(ring: &Ring, streak: &StreakSummary) -> String {
    let label = ring_label(ring.config, streak);
    let number_size = centre_text_size(&label, ring);
    format!(
        "{}{}{}{}",
        heat_ring(ring, &streak.recent_daily_counts),
        text_middle(
            ring.cx,
            ring.cy + number_size as u32 / 3,
            number_size,
            600,
            ring.theme.ink,
            &label,
        ),
        text_middle(
            ring.cx,
            ring.cy + ring.radius + RING_CAPTION_OFFSET,
            11.5,
            400,
            ring.theme.muted,
            &ring_caption(ring.config, streak.recent_daily_counts.len()),
        ),
        text_middle(
            ring.cx,
            ring.cy + ring.radius + RING_DATE_OFFSET,
            10.5,
            400,
            ring.theme.muted,
            &date_range(&streak.window_start, &streak.window_end, ring.compact),
        ),
    )
}

fn centre_text_size(label: &str, ring: &Ring) -> f32 {
    let base = if ring.compact { 22.0 } else { 26.0 };
    let band = if ring.compact { 8.0 } else { 10.0 };
    let inner = (ring.radius as f32 - band / 2.0) * 2.0 - 6.0;
    let width_per_point = label.chars().count() as f32 * 0.54;

    (inner / width_per_point).clamp(9.0, base)
}

fn ring_caption(config: &HeatRing, span: usize) -> String {
    match config.window {
        HeatWindow::Streak => "Current Streak".to_owned(),
        HeatWindow::Fixed(_) => format!("Last {span} Days"),
    }
}

fn ring_label(config: &HeatRing, streak: &StreakSummary) -> String {
    let window = &streak.recent_daily_counts;
    let active = window.iter().filter(|count| **count > 0).count();

    config
        .label_template()
        .replace("{X}", &active.to_string())
        .replace("{Y}", &window.len().to_string())
        .replace("{Z}", &streak.current.to_string())
}

fn heat_ring(ring: &Ring, counts: &[u32]) -> String {
    if counts.is_empty() {
        return format!(
            r#"<circle cx="{cx}" cy="{cy}" r="{radius}" fill="none" stroke="{track}" stroke-width="3"/>"#,
            cx = ring.cx,
            cy = ring.cy,
            radius = ring.radius,
            track = ring.theme.track,
        );
    }

    match ring.config.shape {
        HeatShape::Ticks => heat_tick_ring(ring, counts),
        HeatShape::Arcs => heat_band_ring(ring, counts, false),
        HeatShape::Bands => heat_band_ring(ring, counts, true),
        HeatShape::Segmented => {
            if counts.len() as u32 > ring.config.threshold {
                heat_band_ring(ring, counts, true)
            } else {
                heat_tick_ring(ring, counts)
            }
        }
    }
}

fn heat_tick_ring(ring: &Ring, counts: &[u32]) -> String {
    let length = if ring.compact { 8.0 } else { 10.0 };
    let width = if ring.compact { 3.0 } else { 3.6 };
    let levels = heat_levels(counts, ring.config.scale);
    let step = 360.0 / counts.len() as f64;
    let inner = f64::from(ring.radius) - length / 2.0;
    let outer = f64::from(ring.radius) + length / 2.0;

    levels
        .iter()
        .enumerate()
        .map(|(index, level)| {
            let angle = (-90.0 + index as f64 * step).to_radians();
            let (sin, cos) = angle.sin_cos();
            format!(
                r#"<path d="M{x0:.2} {y0:.2}L{x1:.2} {y1:.2}" stroke="{color}" stroke-width="{width}" stroke-linecap="round"/>"#,
                x0 = f64::from(ring.cx) + inner * cos,
                y0 = f64::from(ring.cy) + inner * sin,
                x1 = f64::from(ring.cx) + outer * cos,
                y1 = f64::from(ring.cy) + outer * sin,
                color = level_color(*level, ring),
                width = width,
            )
        })
        .collect()
}

fn heat_band_ring(ring: &Ring, counts: &[u32], average: bool) -> String {
    let thickness = if ring.compact { 8.0 } else { 10.0 };
    let radius = f64::from(ring.radius);
    let bands = if average {
        band_heat(counts, radius)
    } else {
        counts.to_vec()
    };
    let levels = heat_levels(&bands, ring.config.scale);
    let step = 360.0 / bands.len() as f64;

    levels
        .iter()
        .enumerate()
        .map(|(index, level)| {
            let start = (-90.0 + index as f64 * step).to_radians();
            let end = (-90.0 + (index as f64 + 1.35) * step).to_radians();
            format!(
                r#"<path d="M{x0:.2} {y0:.2}A{radius} {radius} 0 0 1 {x1:.2} {y1:.2}" fill="none" stroke="{color}" stroke-width="{thickness}"/>"#,
                x0 = f64::from(ring.cx) + radius * start.cos(),
                y0 = f64::from(ring.cy) + radius * start.sin(),
                x1 = f64::from(ring.cx) + radius * end.cos(),
                y1 = f64::from(ring.cy) + radius * end.sin(),
                radius = radius,
                color = level_color(*level, ring),
                thickness = thickness,
            )
        })
        .collect()
}

fn band_heat(counts: &[u32], radius: f64) -> Vec<u32> {
    let capacity = (std::f64::consts::TAU * radius / MINIMUM_BAND_WIDTH) as usize;
    let bands = capacity.clamp(1, counts.len());

    (0..bands)
        .map(|index| {
            let start = index * counts.len() / bands;
            let end = ((index + 1) * counts.len() / bands).max(start + 1);
            let slice = &counts[start..end.min(counts.len())];
            let total: u32 = slice.iter().sum();
            total.div_ceil(slice.len() as u32)
        })
        .collect()
}

fn heat_levels(counts: &[u32], scale: HeatScale) -> Vec<Option<usize>> {
    let top = HEAT_RAMP_STEPS - 1;
    let peak = counts.iter().copied().max().unwrap_or(0);
    let steps = HEAT_RAMP_STEPS as f64;

    let cuts = if scale == HeatScale::Quantile {
        let mut active = counts
            .iter()
            .copied()
            .filter(|count| *count > 0)
            .collect::<Vec<_>>();
        active.sort_unstable();
        (1..HEAT_RAMP_STEPS)
            .filter_map(|index| active.get(active.len() * index / HEAT_RAMP_STEPS).copied())
            .collect()
    } else {
        Vec::new()
    };

    counts
        .iter()
        .map(|count| {
            if *count == 0 {
                return None;
            }
            if peak <= 1 {
                return Some(top);
            }
            let span = f64::from(peak - 1);
            let offset = f64::from(count - 1);
            let level = match scale {
                HeatScale::Linear => (offset / span * steps) as usize,
                HeatScale::Sqrt => ((offset / span).sqrt() * steps) as usize,
                HeatScale::Log => (f64::from(*count).ln() / f64::from(peak).ln() * steps) as usize,
                HeatScale::Quantile => cuts.iter().filter(|cut| *count > **cut).count(),
            };
            Some(level.min(top))
        })
        .collect()
}

fn level_color<'a>(level: Option<usize>, ring: &'a Ring) -> &'a str {
    level.map_or(ring.theme.track, |index| {
        ring.stops[index.min(ring.stops.len() - 1)].as_str()
    })
}

fn activity_section(area: Rect, summary: &CodingActivitySummary, theme: &RenderTheme) -> String {
    let step = (area.height.saturating_sub(52) / 5).clamp(18, 26);
    let rows = summary
        .entries
        .iter()
        .take(5)
        .enumerate()
        .map(|(index, entry)| {
            let y = area.y + 52 + index as u32 * step;
            format!(
                "{}{}",
                text(area.x, y, 12.5, theme.ink, &entry.language),
                text_end(
                    area.right(),
                    y,
                    12.5,
                    theme.muted,
                    &format_duration(entry.seconds)
                ),
            )
        })
        .collect::<String>();

    format!(
        "{}{}",
        eyebrow(area.x, area.y + 16, "Coding Activity", theme),
        rows
    )
}

fn eyebrow(x: u32, y: u32, value: &str, theme: &RenderTheme) -> String {
    format!(
        r#"<text x="{}" y="{}" font-size="10.5" font-weight="600" fill="{}" letter-spacing="0.09em">{}</text>"#,
        x,
        y,
        theme.muted,
        escape_xml(&value.to_uppercase()),
    )
}

fn text(x: u32, y: u32, size: f32, fill: &str, value: &str) -> String {
    text_weighted(x, y, size, 400, fill, value)
}

fn text_weighted(x: u32, y: u32, size: f32, weight: u32, fill: &str, value: &str) -> String {
    format!(
        r#"<text x="{}" y="{}" font-size="{}" font-weight="{}" fill="{}">{}</text>"#,
        x,
        y,
        size,
        weight,
        fill,
        escape_xml(value),
    )
}

fn text_end(x: u32, y: u32, size: f32, fill: &str, value: &str) -> String {
    format!(
        r#"<text x="{}" y="{}" font-size="{}" font-weight="600" fill="{}" text-anchor="end">{}</text>"#,
        x,
        y,
        size,
        fill,
        escape_xml(value),
    )
}

fn text_middle(x: u32, y: u32, size: f32, weight: u32, fill: &str, value: &str) -> String {
    format!(
        r#"<text x="{}" y="{}" font-size="{}" font-weight="{}" fill="{}" text-anchor="middle">{}</text>"#,
        x,
        y,
        size,
        weight,
        fill,
        escape_xml(value),
    )
}

fn horizontal_rule(x1: u32, x2: u32, y: u32, theme: &RenderTheme) -> String {
    format!(
        r#"<path d="M{} {}.5H{}" stroke="{}" stroke-width="1"/>"#,
        x1, y, x2, theme.line
    )
}

fn vertical_rule(x: u32, y1: u32, y2: u32, theme: &RenderTheme) -> String {
    format!(
        r#"<path d="M{}.5 {}V{}" stroke="{}" stroke-width="1"/>"#,
        x, y1, y2, theme.line
    )
}

fn rounded_rect(x: u32, y: u32, width: u32, height: u32, fill: &str) -> String {
    format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}"/>"#,
        x,
        y,
        width,
        height,
        height / 2,
        fill,
    )
}

fn language_dot(cx: u32, cy: u32, radius: f32, language: &LanguageShare, index: usize) -> String {
    format!(
        r#"<circle cx="{}" cy="{}" r="{}" fill="{}"/>"#,
        cx,
        cy,
        radius,
        language_color(&language.name, index),
    )
}

fn badge(x: u32, y: u32, value: &str, theme: &RenderTheme) -> String {
    let width = (value.chars().count() as u32 * 9).max(64) + 28;
    format!(
        r#"{}<text x="{}" y="{}" font-size="12.5" font-weight="600" fill="{}">{}</text>"#,
        rounded_rect(x, y, width, 28, theme.accent),
        x + 14,
        y + 19,
        theme.on_accent,
        escape_xml(value),
    )
}

fn share(language: &LanguageShare) -> String {
    format!(
        "{:.1}%",
        f64::from(language.percentage_basis_points) / 100.0
    )
}

enum IconKind {
    Star,
    Commit,
    PullRequest,
    Issue,
    Review,
    Repository,
}

fn icon(kind: IconKind, x: u32, y: u32, size: u32, color: &str) -> String {
    format!(
        r#"<svg x="{}" y="{}" width="{}" height="{}" viewBox="0 0 16 16" fill="none" stroke="{}" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{}</svg>"#,
        x,
        y,
        size,
        size,
        color,
        icon_markup(kind, color),
    )
}

fn icon_markup(kind: IconKind, color: &str) -> String {
    match kind {
        IconKind::Star => {
            r#"<path d="M8 2.6l1.72 3.48 3.84.56-2.78 2.71.66 3.83L8 11.37l-3.44 1.81.66-3.83-2.78-2.71 3.84-.56z"/>"#.to_owned()
        }
        IconKind::Commit => {
            r#"<circle cx="8" cy="8" r="2.4"/><path d="M1.4 8h4.2M10.4 8h4.2"/>"#.to_owned()
        }
        IconKind::PullRequest => {
            r#"<circle cx="4.9" cy="4" r="1.8"/><circle cx="4.9" cy="12" r="1.8"/><circle cx="11.1" cy="4" r="1.8"/><path d="M4.9 5.8v4.4M11.1 5.8v1.9c0 2.1-1.7 2.9-3.7 3.2"/>"#.to_owned()
        }
        IconKind::Issue => format!(
            r#"<circle cx="8" cy="8" r="5.7"/><path d="M8 4.9v3.4"/><circle cx="8" cy="11.1" r="0.85" fill="{color}" stroke="none"/>"#
        ),
        IconKind::Review => {
            r#"<rect x="2.4" y="3.2" width="11.2" height="7.6" rx="1.6"/><path d="M5.6 13.3l2.3-2.5"/><path d="M5.9 6.8l1.7 1.7 2.6-3"/>"#.to_owned()
        }
        IconKind::Repository => {
            r#"<rect x="3.1" y="2.6" width="9.8" height="10.8" rx="1.5"/><path d="M6.1 2.6v6.7l1.9-1.3 1.9 1.3V2.6"/>"#.to_owned()
        }
    }
}

fn format_number(value: u64) -> String {
    let digits = value.to_string();
    let mut formatted = String::new();
    for (index, character) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(character);
    }
    formatted.chars().rev().collect()
}

fn date_range(start: &Option<String>, end: &Option<String>, compact: bool) -> String {
    let format = |value: &str| {
        if compact {
            format_short_date(value)
        } else {
            format_single_date(value)
        }
    };

    match (start.as_deref(), end.as_deref()) {
        (Some(start), Some(end)) if start == end => format(start),
        (Some(start), Some(end)) => format!("{} – {}", format(start), format(end)),
        _ => String::new(),
    }
}

const MONTH_NAMES: [&str; 13] = [
    "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn format_single_date(date: &str) -> String {
    let mut parts = date.split('-');
    let year = parts.next().unwrap_or(date);
    let month = parts
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let day = parts.next().unwrap_or("");
    let Some(month_name) = MONTH_NAMES
        .get(month)
        .copied()
        .filter(|name| !name.is_empty())
    else {
        return date.to_owned();
    };

    format!("{month_name} {} {year}", day.trim_start_matches('0'))
}

fn format_short_date(date: &str) -> String {
    let full = format_single_date(date);
    if full == date {
        return full;
    }
    full.rsplit_once(' ')
        .map(|(head, _)| head.to_owned())
        .unwrap_or(full)
}

fn language_color(name: &str, fallback_index: usize) -> &'static str {
    match name {
        "Assembly" => "#6E4C13",
        "C" => "#555555",
        "C#" => "#178600",
        "C++" => "#f34b7d",
        "CSS" => "#563d7c",
        "Clojure" => "#db5855",
        "Dart" => "#00B4AB",
        "Dockerfile" => "#384d54",
        "Elixir" => "#6e4a7e",
        "Emacs Lisp" => "#c065db",
        "Go" => "#00ADD8",
        "HTML" => "#e34c26",
        "Haskell" => "#5e5086",
        "Java" => "#b07219",
        "JavaScript" => "#f1e05a",
        "Kotlin" => "#A97BFF",
        "Lua" => "#000080",
        "Makefile" => "#427819",
        "Nix" => "#7e7eff",
        "PHP" => "#4F5D95",
        "Python" => "#3572A5",
        "Ruby" => "#701516",
        "Rust" => "#dea584",
        "Scala" => "#c22d40",
        "Shell" => "#89e051",
        "Swift" => "#F05138",
        "TypeScript" => "#3178c6",
        "Vim Script" => "#199f4b",
        "Vue" => "#41b883",
        "Zig" => "#ec915c",
        _ => fallback_language_color(fallback_index),
    }
}

fn fallback_language_color(index: usize) -> &'static str {
    ["#6f42c1", "#0969da", "#1a7f37", "#fb8500", "#d63384"]
        .get(index)
        .copied()
        .unwrap_or("#57606a")
}

fn progress_bar(seconds: u64, total: u64) -> String {
    let filled = seconds
        .saturating_mul(10)
        .checked_div(total)
        .unwrap_or(0)
        .min(10);
    let empty = 10 - filled;
    format!(
        "{}{} {}",
        "█".repeat(filled as usize),
        "░".repeat(empty as usize),
        format_duration(seconds)
    )
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = seconds % 3600 / 60;
    format!("{hours} hrs {minutes} mins")
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_markdown(value: &str) -> String {
    value.replace('<', "&lt;").replace('>', "&gt;")
}
