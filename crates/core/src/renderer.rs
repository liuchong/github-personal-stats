use crate::{
    AggregatedStats, CardData, CodingActivitySummary, GithubStatsConfig, ImageSize, LanguageShare,
    StreakSummary,
};

const FONT_STACK: &str =
    "-apple-system, BlinkMacSystemFont, 'Segoe UI', Inter, 'Helvetica Neue', Arial, sans-serif";

const HEAT_RAMP: [&str; 4] = ["#ffe3ad", "#ffc65c", "#ffa726", "#fb8c00"];

const NARROW_WIDTH: u32 = 440;

const GUTTER: u32 = 24;
const LANGUAGE_ROWS: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTheme {
    pub background: &'static str,
    pub ink: &'static str,
    pub muted: &'static str,
    pub line: &'static str,
    pub track: &'static str,
    pub accent: &'static str,
    pub on_accent: &'static str,
}

impl RenderTheme {
    pub fn named(name: &str) -> Self {
        match name {
            "dark" => Self {
                background: "#0d1117",
                ink: "#e6edf3",
                muted: "#8b949e",
                line: "#21262d",
                track: "#262c36",
                accent: "#4493f8",
                on_accent: "#0d1117",
            },
            "transparent" => Self {
                background: "transparent",
                ink: "#1f2328",
                muted: "#59636e",
                line: "#d8dee6",
                track: "#dde3ea",
                accent: "#0969da",
                on_accent: "#ffffff",
            },
            _ => Self {
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
        self.width < NARROW_WIDTH
    }
}

pub fn render_card(card: &CardData, config: &GithubStatsConfig) -> String {
    let theme = RenderTheme::named(&config.theme);
    match card {
        CardData::Dashboard {
            stats,
            languages,
            streak,
        } => render_dashboard(stats, languages, streak, &config.size, &theme),
        CardData::Stats(stats) => render_stats_card(stats, &config.size, &theme),
        CardData::Languages(languages) => render_languages_card(languages, &config.size, &theme),
        CardData::Streak(streak) => render_streak_card(streak, &config.size, &theme),
        CardData::Wakatime(summary) => render_wakatime_card(summary, &config.size, &theme),
        CardData::Status { state } => render_status_card(state, &config.size, &theme),
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
    size: &ImageSize,
    theme: &RenderTheme,
) -> String {
    let pad = padding(size.width);
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
        theme,
        format!(
            "{}{}{}{}{}",
            stats_section(stats_area, stats, theme),
            vertical_rule(pad + column + GUTTER / 2, pad + 2, pad + top_height, theme),
            languages_section(languages_area, languages, theme),
            horizontal_rule(pad, size.width - pad, split, theme),
            streak_section(streak_area, streak, theme),
        ),
    )
}

fn render_stats_card(stats: &AggregatedStats, size: &ImageSize, theme: &RenderTheme) -> String {
    svg_root(size, theme, stats_section(card_area(size), stats, theme))
}

fn render_languages_card(
    languages: &[LanguageShare],
    size: &ImageSize,
    theme: &RenderTheme,
) -> String {
    svg_root(
        size,
        theme,
        languages_section(card_area(size), languages, theme),
    )
}

fn render_streak_card(streak: &StreakSummary, size: &ImageSize, theme: &RenderTheme) -> String {
    svg_root(size, theme, streak_section(card_area(size), streak, theme))
}

fn render_wakatime_card(
    summary: &CodingActivitySummary,
    size: &ImageSize,
    theme: &RenderTheme,
) -> String {
    svg_root(
        size,
        theme,
        wakatime_section(card_area(size), summary, theme),
    )
}

fn render_status_card(state: &str, size: &ImageSize, theme: &RenderTheme) -> String {
    let area = card_area(size);
    svg_root(
        size,
        theme,
        format!(
            "{}{}{}",
            eyebrow(area.x, area.y + 16, "Status", theme),
            badge(area.x, area.y + 44, state, theme),
            text(area.x, area.y + 96, 11.0, theme.muted, "Service health"),
        ),
    )
}

fn card_area(size: &ImageSize) -> Rect {
    let pad = padding(size.width);
    Rect {
        x: pad,
        y: pad,
        width: size.width.saturating_sub(pad * 2),
        height: size.height.saturating_sub(pad * 2),
    }
}

fn padding(width: u32) -> u32 {
    (width / 20).clamp(16, 28)
}

fn svg_root(size: &ImageSize, theme: &RenderTheme, body: String) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" shape-rendering="geometricPrecision" text-rendering="optimizeLegibility" font-family="{font}" style="font-variant-numeric:tabular-nums"><rect width="100%" height="100%" fill="{background}"/>{body}</svg>"#,
        width = size.width,
        height = size.height,
        font = FONT_STACK,
        background = theme.background,
        body = body,
    )
}

fn stats_section(area: Rect, stats: &AggregatedStats, theme: &RenderTheme) -> String {
    let radius = (area.width / 12).clamp(20, 38);
    let ring_cx = area.right().saturating_sub(radius + 10);
    let ring_cy = area.y + 34 + area.height.saturating_sub(34) / 2 - 10;
    let value_x = ring_cx.saturating_sub(radius + 30);
    let step = (area.height.saturating_sub(58) / 4).clamp(20, 30);

    let rows = [
        ("Total Stars", stats.total_stars, IconKind::Star),
        ("Commits", stats.total_commits, IconKind::Commit),
        (
            "Pull Requests",
            stats.total_pull_requests,
            IconKind::PullRequest,
        ),
        ("Issues", stats.total_issues, IconKind::Issue),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (label, value, icon_kind))| {
        stat_row(
            area.x,
            area.y + 54 + index as u32 * step,
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
            10.0,
            400,
            theme.muted,
            &format!("RANK · {}", format_number(stats.score)),
        ),
    )
}

fn languages_section(area: Rect, languages: &[LanguageShare], theme: &RenderTheme) -> String {
    let rows = if area.is_narrow() {
        language_track_rows(area, languages, theme)
    } else {
        language_columns(area, languages, theme)
    };

    format!(
        "{}{}{}",
        eyebrow(area.x, area.y + 16, "Languages", theme),
        stacked_language_bar(area.x, area.y + 32, area.width, languages, theme),
        rows,
    )
}

fn language_columns(area: Rect, languages: &[LanguageShare], theme: &RenderTheme) -> String {
    let per_column = LANGUAGE_ROWS / 2;
    let column = area.width.saturating_sub(GUTTER) / 2;

    languages
        .iter()
        .take(LANGUAGE_ROWS)
        .enumerate()
        .map(|(index, language)| {
            let x = area.x + (index / per_column) as u32 * (column + GUTTER);
            let y = area.y + 72 + (index % per_column) as u32 * 24;
            format!(
                "{}{}{}",
                language_dot(x + 4, y - 4, 4.0, language, index),
                text(x + 16, y, 12.0, theme.ink, &language.name),
                text_end(x + column, y, 12.0, theme.muted, &share(language)),
            )
        })
        .collect()
}

fn language_track_rows(area: Rect, languages: &[LanguageShare], theme: &RenderTheme) -> String {
    let name_column = (area.width * 28 / 100).clamp(90, 150);
    let track_x = area.x + name_column;
    let track_width = area
        .width
        .saturating_sub(name_column + 52)
        .max(TRACK_MINIMUM);
    let step = (area.height.saturating_sub(56) / LANGUAGE_ROWS as u32).clamp(15, 20);

    languages
        .iter()
        .take(LANGUAGE_ROWS)
        .enumerate()
        .map(|(index, language)| {
            let y = area.y + 56 + index as u32 * step;
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
) -> String {
    let mut consumed_basis_points = 0;
    let mut previous_edge = 0;
    let mut segments = String::new();

    for (index, language) in languages.iter().take(LANGUAGE_ROWS).enumerate() {
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

fn streak_section(area: Rect, streak: &StreakSummary, theme: &RenderTheme) -> String {
    let column = area.width / 3;
    let compact = area.is_narrow();
    let (label_y, value_y, note_y) = if compact {
        (area.y + 44, area.y + 80, area.y + 104)
    } else {
        (area.y + 52, area.y + 92, area.y + 118)
    };
    let value_size = if compact { 26.0 } else { 34.0 };
    let ring_radius = if compact { 26 } else { 32 };
    let ring_cy = area.y + if compact { 62 } else { 70 };
    let ring_cx = area.x + column + column / 2;

    let total = side_metric(SideMetric {
        x: area.x,
        label_y,
        value_y,
        note_y,
        value_size,
        label: "Total Contributions",
        value: format_number(streak.total_contributions),
        unit: "",
        note: streak
            .current_end
            .as_deref()
            .map(|date| format!("through {}", format_single_date(date)))
            .unwrap_or_default(),
        theme,
    });
    let longest = side_metric(SideMetric {
        x: area.x + column * 2,
        label_y,
        value_y,
        note_y,
        value_size,
        label: "Longest Streak",
        value: streak.longest.to_string(),
        unit: "days",
        note: date_range(&streak.longest_start, &streak.longest_end, compact),
        theme,
    });

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
        current_streak_ring(ring_cx, ring_cy, ring_radius, compact, streak, theme),
        vertical_rule(
            area.x + column * 2 - 12,
            area.y + 20,
            area.y + area.height,
            theme
        ),
        longest,
    )
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
}

fn side_metric(metric: SideMetric<'_>) -> String {
    let label_size = if metric.value_size > 30.0 { 11.0 } else { 10.0 };
    let unit = if metric.unit.is_empty() {
        String::new()
    } else {
        let advance = metric.value.chars().count() as f32 * metric.value_size * 0.6;
        text(
            metric.x + advance as u32 + 6,
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

fn current_streak_ring(
    cx: u32,
    cy: u32,
    radius: u32,
    compact: bool,
    streak: &StreakSummary,
    theme: &RenderTheme,
) -> String {
    let number_size = if compact { 22.0 } else { 26.0 };
    format!(
        "{}{}{}{}",
        heat_ring(cx, cy, radius, compact, &streak.recent_daily_counts, theme),
        text_middle(
            cx,
            cy + number_size as u32 / 3,
            number_size,
            600,
            theme.ink,
            &streak.current.to_string(),
        ),
        text_middle(
            cx,
            cy + radius + 26,
            11.5,
            400,
            theme.muted,
            "Current Streak"
        ),
        text_middle(
            cx,
            cy + radius + 43,
            10.5,
            400,
            theme.muted,
            &date_range(&streak.current_start, &streak.current_end, compact),
        ),
    )
}

fn heat_ring(
    cx: u32,
    cy: u32,
    radius: u32,
    compact: bool,
    counts: &[u32],
    theme: &RenderTheme,
) -> String {
    if counts.is_empty() {
        return format!(
            r#"<circle cx="{cx}" cy="{cy}" r="{radius}" fill="none" stroke="{track}" stroke-width="3"/>"#,
            cx = cx,
            cy = cy,
            radius = radius,
            track = theme.track,
        );
    }

    let length = if compact { 8.0 } else { 10.0 };
    let width = if compact { 3.0 } else { 3.6 };
    let peak = counts.iter().copied().max().unwrap_or(0);
    let step = 360.0 / counts.len() as f64;
    let inner = f64::from(radius) - length / 2.0;
    let outer = f64::from(radius) + length / 2.0;

    counts
        .iter()
        .enumerate()
        .map(|(index, count)| {
            let angle = (-90.0 + index as f64 * step).to_radians();
            let (sin, cos) = angle.sin_cos();
            format!(
                r#"<path d="M{x0:.2} {y0:.2}L{x1:.2} {y1:.2}" stroke="{color}" stroke-width="{width}" stroke-linecap="round"/>"#,
                x0 = f64::from(cx) + inner * cos,
                y0 = f64::from(cy) + inner * sin,
                x1 = f64::from(cx) + outer * cos,
                y1 = f64::from(cy) + outer * sin,
                color = heat_color(*count, peak, theme),
                width = width,
            )
        })
        .collect()
}

fn heat_color(count: u32, peak: u32, theme: &RenderTheme) -> &str {
    if count == 0 {
        return theme.track;
    }
    if peak <= 1 {
        return HEAT_RAMP[HEAT_RAMP.len() - 1];
    }
    let position = f64::from(count - 1) / f64::from(peak - 1) * HEAT_RAMP.len() as f64;
    HEAT_RAMP[(position as usize).min(HEAT_RAMP.len() - 1)]
}

fn wakatime_section(area: Rect, summary: &CodingActivitySummary, theme: &RenderTheme) -> String {
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
