use crate::{
    AggregatedStats, CardData, CodingActivitySummary, GithubStatsConfig, ImageSize, LanguageShare,
    StreakSummary,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTheme {
    pub background: &'static str,
    pub panel: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub accent: &'static str,
    pub border: &'static str,
    pub accent_soft: &'static str,
    pub success: &'static str,
}

impl RenderTheme {
    pub fn named(name: &str) -> Self {
        match name {
            "dark" => Self {
                background: "#0d1117",
                panel: "#161b22",
                text: "#f0f6fc",
                muted: "#8b949e",
                accent: "#58a6ff",
                border: "#30363d",
                accent_soft: "#102542",
                success: "#3fb950",
            },
            "transparent" => Self {
                background: "transparent",
                panel: "#ffffffcc",
                text: "#24292f",
                muted: "#57606a",
                accent: "#0969da",
                border: "#d0d7de",
                accent_soft: "#ddf4ff",
                success: "#1a7f37",
            },
            _ => Self {
                background: "#f6f8ff",
                panel: "#ffffff",
                text: "#24292f",
                muted: "#57606a",
                accent: "#6f42c1",
                border: "#d8dee8",
                accent_soft: "#f0e7ff",
                success: "#1a7f37",
            },
        }
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
    let padding = 24;
    let gap = 16;
    let top_height = (size.height.saturating_sub(padding * 2 + gap)) / 2;
    let bottom_y = padding + top_height + gap;
    let panel_width = (size.width.saturating_sub(padding * 2 + gap)) / 2;
    let bottom_width = size.width.saturating_sub(padding * 2);

    svg_root(
        size,
        theme,
        format!(
            "{}{}{}{}{}{}",
            panel(
                padding,
                padding,
                panel_width,
                top_height,
                "Stats",
                "Profile overview",
                theme
            ),
            stats_dashboard(stats, padding + 24, padding + 58, panel_width - 48, theme),
            panel(
                padding + panel_width + gap,
                padding,
                panel_width,
                top_height,
                "Languages",
                "Repository language share",
                theme
            ),
            language_bars(
                languages,
                padding + panel_width + gap + 24,
                padding + 58,
                panel_width - 48,
                theme
            ),
            panel(
                padding,
                bottom_y,
                bottom_width,
                top_height,
                "Streak",
                "Recent public activity",
                theme
            ),
            streak_tiles(
                streak,
                padding + 24,
                bottom_y + 62,
                bottom_width - 48,
                theme
            )
        ),
    )
}

fn render_stats_card(stats: &AggregatedStats, size: &ImageSize, theme: &RenderTheme) -> String {
    svg_root(
        size,
        theme,
        panel(
            16,
            16,
            size.width - 32,
            size.height - 32,
            "Stats",
            "Profile overview",
            theme,
        ) + &stats_dashboard(stats, 40, 74, size.width - 80, theme),
    )
}

fn render_languages_card(
    languages: &[LanguageShare],
    size: &ImageSize,
    theme: &RenderTheme,
) -> String {
    svg_root(
        size,
        theme,
        panel(
            16,
            16,
            size.width - 32,
            size.height - 32,
            "Languages",
            "Repository language share",
            theme,
        ) + &language_bars(languages, 40, 74, size.width - 80, theme),
    )
}

fn render_streak_card(streak: &StreakSummary, size: &ImageSize, theme: &RenderTheme) -> String {
    svg_root(
        size,
        theme,
        panel(
            16,
            16,
            size.width - 32,
            size.height - 32,
            "Streak",
            "Recent public activity",
            theme,
        ) + &streak_tiles(streak, 40, 82, size.width - 80, theme),
    )
}

fn render_wakatime_card(
    summary: &CodingActivitySummary,
    size: &ImageSize,
    theme: &RenderTheme,
) -> String {
    svg_root(
        size,
        theme,
        panel(
            16,
            16,
            size.width - 32,
            size.height - 32,
            "Coding Activity",
            "Tracked development time",
            theme,
        ) + &wakatime_lines(summary, 40, 78, theme),
    )
}

fn render_status_card(state: &str, size: &ImageSize, theme: &RenderTheme) -> String {
    svg_root(
        size,
        theme,
        panel(
            16,
            16,
            size.width - 32,
            size.height - 32,
            "Status",
            "Service health",
            theme,
        ) + &badge(40, 78, 160, 34, state, theme.success, "#ffffff"),
    )
}

fn svg_root(size: &ImageSize, theme: &RenderTheme, body: String) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" role="img" shape-rendering="geometricPrecision"><defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="{}"/><stop offset="100%" stop-color="{}"/></linearGradient><filter id="shadow" x="-10%" y="-10%" width="120%" height="130%"><feDropShadow dx="0" dy="4" stdDeviation="10" flood-color="#1f2937" flood-opacity="0.08"/></filter></defs><rect width="100%" height="100%" fill="url(#bg)"/>{}</svg>"##,
        size.width, size.height, size.width, size.height, theme.background, theme.accent_soft, body
    )
}

fn panel(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    title: &str,
    subtitle: &str,
    theme: &RenderTheme,
) -> String {
    format!(
        r#"<g filter="url(#shadow)"><rect x="{}" y="{}" width="{}" height="{}" rx="18" fill="{}" stroke="{}"/><rect x="{}" y="{}" width="2.5" height="34" rx="1.25" fill="{}"/><text x="{}" y="{}" font-family="'Helvetica Neue', Arial, sans-serif" font-size="19" font-weight="500" fill="{}">{}</text><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="12" fill="{}">{}</text></g>"#,
        x,
        y,
        width,
        height,
        theme.panel,
        theme.border,
        x + 20,
        y + 22,
        theme.accent,
        x + 36,
        y + 32,
        theme.text,
        escape_xml(title),
        x + 36,
        y + 50,
        theme.muted,
        escape_xml(subtitle)
    )
}

fn stats_dashboard(
    stats: &AggregatedStats,
    x: u32,
    y: u32,
    width: u32,
    theme: &RenderTheme,
) -> String {
    let list_width = width.saturating_sub(150);
    [
        stat_row(
            x,
            y,
            list_width,
            "Total Stars",
            stats.total_stars,
            theme.accent,
            theme,
        ),
        stat_row(
            x,
            y + 28,
            list_width,
            "Commits",
            stats.total_commits,
            theme.success,
            theme,
        ),
        stat_row(
            x,
            y + 56,
            list_width,
            "Pull Requests",
            stats.total_pull_requests,
            "#fb8500",
            theme,
        ),
        stat_row(
            x,
            y + 84,
            list_width,
            "Issues",
            stats.total_issues,
            "#d63384",
            theme,
        ),
        rank_ring(x + width - 118, y + 4, 96, stats.rank, stats.score, theme),
    ]
    .join("")
}

fn stat_row<T: ToString>(
    x: u32,
    y: u32,
    width: u32,
    label: &str,
    value: T,
    accent: &str,
    theme: &RenderTheme,
) -> String {
    format!(
        r#"<g>{}<text x="{}" y="{}" font-family="'Helvetica Neue', Arial, sans-serif" font-size="12" font-weight="500" fill="{}">{}</text><text x="{}" y="{}" text-anchor="end" font-family="'Helvetica Neue', Arial, sans-serif" font-size="14" font-weight="500" fill="{}">{}</text></g>"#,
        icon(stat_icon(label), x, y, 16, accent),
        x + 22,
        y + 13,
        theme.text,
        escape_xml(label),
        x + width,
        y + 13,
        theme.text,
        escape_xml(&value.to_string())
    )
}

fn rank_ring(x: u32, y: u32, size: u32, rank: &str, score: u64, theme: &RenderTheme) -> String {
    let center = size / 2;
    format!(
        r#"<g><circle cx="{}" cy="{}" r="42" fill="{}" stroke="{}" stroke-width="2.5"/><circle cx="{}" cy="{}" r="42" fill="none" stroke="{}" stroke-width="2.5" stroke-linecap="round" stroke-dasharray="205 264" transform="rotate(-90 {} {})"/><text x="{}" y="{}" text-anchor="middle" font-family="Arial, sans-serif" font-size="30" font-weight="700" fill="{}">{}</text><text x="{}" y="{}" text-anchor="middle" font-family="Arial, sans-serif" font-size="10" font-weight="600" fill="{}">RANK</text><text x="{}" y="{}" text-anchor="middle" font-family="Arial, sans-serif" font-size="10" fill="{}">score {}</text></g>"#,
        x + center,
        y + center,
        theme.panel,
        theme.accent_soft,
        x + center,
        y + center,
        rank_color(rank, theme),
        x + center,
        y + center,
        x + center,
        y + center + 8,
        theme.text,
        escape_xml(rank),
        x + center,
        y + center - 22,
        theme.muted,
        x + center,
        y + center + 27,
        theme.muted,
        score
    )
}

fn language_bars(
    languages: &[LanguageShare],
    x: u32,
    y: u32,
    width: u32,
    theme: &RenderTheme,
) -> String {
    let bar_width = width.saturating_sub(150);
    let rows = languages
        .iter()
        .take(6)
        .enumerate()
        .map(|(index, language)| {
            let row_y = y + 24 + index as u32 * 17;
            let percentage = language.percentage_basis_points as f32 / 100.0;
            let filled = bar_width * language.percentage_basis_points / 10_000;
            let color = language_color(&language.name, index);
            format!(
                r#"{}{}{}{}{}"#,
                icon(IconKind::Code, x, row_y, 12, color),
                text(x + 18, row_y + 8, 11, theme.text, &language.name),
                text(
                    x + width - 58,
                    row_y + 8,
                    11,
                    theme.muted,
                    &format!("{percentage:.1}%")
                ),
                rounded_rect(
                    x + 94,
                    row_y + 2,
                    bar_width,
                    4,
                    2,
                    theme.accent_soft,
                    "none"
                ),
                rounded_rect(x + 94, row_y + 2, filled, 4, 2, color, "none")
            )
        })
        .collect::<String>();

    stacked_language_bar(languages, x, y, width, theme) + &rows
}

fn stacked_language_bar(
    languages: &[LanguageShare],
    x: u32,
    y: u32,
    width: u32,
    theme: &RenderTheme,
) -> String {
    let mut offset = 0;
    let total_width = width.saturating_sub(4);
    let mut segments = String::new();

    for (index, language) in languages.iter().take(6).enumerate() {
        let segment_width = if index == 5 {
            total_width.saturating_sub(offset)
        } else {
            total_width * language.percentage_basis_points / 10_000
        };
        segments.push_str(&rounded_rect(
            x + 2 + offset,
            y,
            segment_width,
            6,
            3,
            language_color(&language.name, index),
            "none",
        ));
        offset += segment_width;
    }

    rounded_rect(x, y, width, 6, 3, theme.accent_soft, "none") + &segments
}

fn streak_tiles(streak: &StreakSummary, x: u32, y: u32, width: u32, theme: &RenderTheme) -> String {
    let compact = width < 640;
    let hero_width = if compact { 160 } else { 260 };
    let side_width = width.saturating_sub(hero_width + 32) / 2;
    let center_x = x + side_width + 16;
    [
        side_streak_metric(SideStreakMetric {
            x,
            y: y + 12,
            width: side_width,
            label: "Total Contributions",
            value: format_number(streak.total_contributions),
            unit: "",
            note: streak
                .current_end
                .as_deref()
                .map(|date| {
                    if compact {
                        format_short_date(date)
                    } else {
                        format_single_date(date)
                    }
                })
                .unwrap_or_default(),
            accent: theme.accent,
            compact,
            theme,
        }),
        current_streak_hero(center_x, y, hero_width, streak, compact, theme),
        side_streak_metric(SideStreakMetric {
            x: center_x + hero_width + 16,
            y: y + 12,
            width: side_width,
            label: "Longest Streak",
            value: streak.longest.to_string(),
            unit: "days",
            note: if compact {
                short_date_range(&streak.longest_start, &streak.longest_end)
            } else {
                date_range(&streak.longest_start, &streak.longest_end)
            },
            accent: theme.success,
            compact,
            theme,
        }),
    ]
    .join("")
}

struct SideStreakMetric<'a> {
    x: u32,
    y: u32,
    width: u32,
    label: &'a str,
    value: String,
    unit: &'a str,
    note: String,
    accent: &'a str,
    compact: bool,
    theme: &'a RenderTheme,
}

fn side_streak_metric(metric: SideStreakMetric<'_>) -> String {
    if metric.compact {
        let (label_top, label_bottom) = metric.label.split_once(' ').unwrap_or((metric.label, ""));
        let unit_x = metric.x + 20 + metric.value.len() as u32 * 12;
        return format!(
            r#"<g><rect x="{}" y="{}" width="{}" height="82" rx="16" fill="{}" stroke="{}"/><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="10" fill="{}">{}</text><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="10" fill="{}">{}</text><text x="{}" y="{}" font-family="'Helvetica Neue', Arial, sans-serif" font-size="24" font-weight="500" fill="{}">{}</text><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="10" fill="{}">{}</text><rect x="{}" y="{}" width="{}" height="1.5" rx="0.75" fill="{}"/><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="9" fill="{}">{}</text></g>"#,
            metric.x,
            metric.y,
            metric.width,
            metric.theme.accent_soft,
            metric.theme.border,
            metric.x + 12,
            metric.y + 17,
            metric.theme.muted,
            escape_xml(label_top),
            metric.x + 12,
            metric.y + 29,
            metric.theme.muted,
            escape_xml(label_bottom),
            metric.x + 12,
            metric.y + 55,
            metric.theme.text,
            escape_xml(&metric.value),
            unit_x,
            metric.y + 55,
            metric.theme.muted,
            escape_xml(metric.unit),
            metric.x + 12,
            metric.y + 63,
            metric.width.saturating_sub(24),
            metric.accent,
            metric.x + 12,
            metric.y + 76,
            metric.theme.muted,
            escape_xml(&metric.note)
        );
    }
    format!(
        r#"<g><rect x="{}" y="{}" width="{}" height="82" rx="16" fill="{}" stroke="{}"/><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="12" fill="{}">{}</text><text x="{}" y="{}" font-family="'Helvetica Neue', Arial, sans-serif" font-size="32" font-weight="500" fill="{}">{}</text><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="11" fill="{}">{}</text><rect x="{}" y="{}" width="{}" height="1.5" rx="0.75" fill="{}"/><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="10" fill="{}">{}</text></g>"#,
        metric.x,
        metric.y,
        metric.width,
        metric.theme.accent_soft,
        metric.theme.border,
        metric.x + 18,
        metric.y + 22,
        metric.theme.muted,
        escape_xml(metric.label),
        metric.x + 18,
        metric.y + 55,
        metric.theme.text,
        escape_xml(&metric.value),
        metric.x + 76,
        metric.y + 55,
        metric.theme.muted,
        escape_xml(metric.unit),
        metric.x + 18,
        metric.y + 62,
        metric.width.saturating_sub(36),
        metric.accent,
        metric.x + 18,
        metric.y + 76,
        metric.theme.muted,
        escape_xml(&metric.note)
    )
}

fn current_streak_hero(
    x: u32,
    y: u32,
    width: u32,
    streak: &StreakSummary,
    compact: bool,
    theme: &RenderTheme,
) -> String {
    let center_x = x + width / 2;
    let radius: u32 = if compact { 26 } else { 34 };
    let ring_cy = y + 4 + radius;
    let ring_top = ring_cy - radius;
    let number_y = ring_cy + if compact { 8 } else { 10 };
    let label_y = ring_cy + radius + if compact { 14 } else { 18 };
    let date_y = label_y + if compact { 13 } else { 16 };
    let number_size = if compact { 24 } else { 34 };
    let label_size = if compact { 11 } else { 14 };
    let date_size = if compact { 9 } else { 12 };
    let flame_color = "#fb8c00";
    let mask_id = "psm-streak-flame-cut";
    let range = date_range(&streak.current_start, &streak.current_end);
    format!(
        r##"<g><defs><mask id="{mask_id}" maskUnits="userSpaceOnUse"><rect x="-1000" y="-1000" width="6000" height="6000" fill="white"/><ellipse cx="{center_x}" cy="{notch_y}" rx="10" ry="15" fill="black"/></mask></defs><circle cx="{center_x}" cy="{ring_cy}" r="{radius}" fill="{panel}" stroke="{accent_soft}" stroke-width="2" mask="url(#{mask_id})"/><circle cx="{center_x}" cy="{ring_cy}" r="{radius}" fill="none" stroke="{flame_color}" stroke-width="2" mask="url(#{mask_id})"/>{flame}<text x="{center_x}" y="{number_y}" text-anchor="middle" font-family="Arial, sans-serif" font-size="{number_size}" font-weight="800" fill="{text_color}">{count}</text><text x="{center_x}" y="{label_y}" text-anchor="middle" font-family="Arial, sans-serif" font-size="{label_size}" font-weight="700" fill="{flame_color}">Current Streak</text><text x="{center_x}" y="{date_y}" text-anchor="middle" font-family="Arial, sans-serif" font-size="{date_size}" fill="{muted}">{range}</text></g>"##,
        mask_id = mask_id,
        center_x = center_x,
        notch_y = ring_top.saturating_sub(6),
        ring_cy = ring_cy,
        radius = radius,
        panel = theme.panel,
        accent_soft = theme.accent_soft,
        flame_color = flame_color,
        flame = flame_icon(center_x, ring_top.saturating_sub(11), flame_color),
        number_y = number_y,
        number_size = number_size,
        text_color = theme.text,
        count = streak.current,
        label_y = label_y,
        label_size = label_size,
        date_y = date_y,
        date_size = date_size,
        muted = theme.muted,
        range = escape_xml(&range)
    )
}

fn rank_color(rank: &str, theme: &RenderTheme) -> &'static str {
    match rank {
        "S+" | "S" => "#ff9800",
        "A+" | "A" | "A-" => theme.accent,
        "B+" | "B" | "B-" => theme.success,
        _ => "#57606a",
    }
}

fn wakatime_lines(summary: &CodingActivitySummary, x: u32, y: u32, theme: &RenderTheme) -> String {
    summary
        .entries
        .iter()
        .take(5)
        .enumerate()
        .map(|(index, entry)| {
            text(
                x,
                y + index as u32 * 24,
                14,
                theme.muted,
                &format!("{} {}", entry.language, format_duration(entry.seconds)),
            )
        })
        .collect()
}

fn badge(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    value: &str,
    fill: &str,
    text_fill: &str,
) -> String {
    format!(
        r#"<g><rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}"/><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="15" font-weight="600" fill="{}">{}</text></g>"#,
        x,
        y,
        width,
        height,
        height / 2,
        fill,
        x + 18,
        y + 23,
        text_fill,
        escape_xml(value)
    )
}

enum IconKind {
    Star,
    Commit,
    PullRequest,
    Issue,
    Code,
}

fn stat_icon(label: &str) -> IconKind {
    match label {
        "Total Stars" => IconKind::Star,
        "Commits" => IconKind::Commit,
        "Pull Requests" => IconKind::PullRequest,
        "Issues" => IconKind::Issue,
        _ => IconKind::Code,
    }
}

fn icon(kind: IconKind, x: u32, y: u32, size: u32, color: &str) -> String {
    format!(
        r#"<svg x="{}" y="{}" width="{}" height="{}" viewBox="0 0 16 16" aria-hidden="true">{}</svg>"#,
        x,
        y,
        size,
        size,
        icon_markup(kind, color)
    )
}

fn icon_markup(kind: IconKind, color: &str) -> String {
    match kind {
        IconKind::Star => format!(
            r#"<path d="M8 2.1l1.7 3.5 3.9.6-2.8 2.7.6 3.8L8 11.1l-3.4 1.6.6-3.8-2.8-2.7 3.9-.6z" fill="none" stroke="{color}" stroke-width="1.2" stroke-linejoin="round"/>"#
        ),
        IconKind::Commit => format!(
            r#"<circle cx="8" cy="8" r="2.4" fill="none" stroke="{color}" stroke-width="1.3"/><path d="M1 8h4.4M10.6 8H15" fill="none" stroke="{color}" stroke-width="1.3" stroke-linecap="round"/>"#
        ),
        IconKind::PullRequest => format!(
            r#"<circle cx="5" cy="3.9" r="1.7" fill="none" stroke="{color}" stroke-width="1.2"/><circle cx="5" cy="12.1" r="1.7" fill="none" stroke="{color}" stroke-width="1.2"/><circle cx="11" cy="3.9" r="1.7" fill="none" stroke="{color}" stroke-width="1.2"/><path d="M5 5.6v4.8M11 5.6v2.2c0 2-1.6 2.7-3.6 3" fill="none" stroke="{color}" stroke-width="1.2" stroke-linecap="round"/>"#
        ),
        IconKind::Issue => format!(
            r#"<circle cx="8" cy="8" r="6" fill="none" stroke="{color}" stroke-width="1.2"/><path d="M8 4.6v3.9" fill="none" stroke="{color}" stroke-width="1.4" stroke-linecap="round"/><circle cx="8" cy="11.2" r="0.9" fill="{color}"/>"#
        ),
        IconKind::Code => format!(
            r#"<path d="M6 4.8 2.8 8l3.2 3.2M10 4.8 13.2 8 10 11.2" fill="none" stroke="{color}" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#
        ),
    }
}

fn rounded_rect(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    radius: u32,
    fill: &str,
    stroke: &str,
) -> String {
    format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}" stroke="{}"/>"#,
        x, y, width, height, radius, fill, stroke
    )
}

fn text(x: u32, y: u32, size: u32, fill: &str, value: &str) -> String {
    format!(
        r#"<text x="{}" y="{}" font-family="Arial, sans-serif" font-size="{}" fill="{}">{}</text>"#,
        x,
        y,
        size,
        fill,
        escape_xml(value)
    )
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

fn date_range(start: &Option<String>, end: &Option<String>) -> String {
    match (start.as_deref(), end.as_deref()) {
        (Some(start), Some(end)) if start == end => format_single_date(start),
        (Some(start), Some(end)) => format!(
            "{} - {}",
            format_single_date(start),
            format_single_date(end)
        ),
        _ => String::new(),
    }
}

fn format_single_date(date: &str) -> String {
    let mut parts = date.split('-');
    let Some(year) = parts.next() else {
        return date.to_owned();
    };
    let month = parts
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let day = parts.next().unwrap_or("");
    let month_name = [
        "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .get(month)
    .copied()
    .unwrap_or("");

    if month_name.is_empty() {
        date.to_owned()
    } else {
        format!("{month_name} {} {}", day.trim_start_matches('0'), year)
    }
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

fn short_date_range(start: &Option<String>, end: &Option<String>) -> String {
    match (start.as_deref(), end.as_deref()) {
        (Some(start), Some(end)) if start == end => format_short_date(start),
        (Some(start), Some(end)) => {
            format!("{} - {}", format_short_date(start), format_short_date(end))
        }
        _ => String::new(),
    }
}

fn flame_icon(cx: u32, base_y: u32, color: &str) -> String {
    let cx = cx as i32;
    let base_y = base_y as i32;
    let flame = "M 1.4 -10.8 C 2 -8 2.6 -5.4 2.5 -3.4 C 2.4 -1.2 1 0.4 -0.9 0.4 C -2.8 0.4 -4.2 -1.2 -4.4 -3 C -6.4 -0.8 -8.2 2.8 -8.2 6.4 C -8.2 10.6 -4.4 13.2 0 13.2 C 4.4 13.2 8.2 10.6 8.2 6.4 C 8.2 1 5.6 -4.4 1.4 -10.8 Z M -0.3 10.4 C -2.1 10.4 -3.5 9 -3.5 7.3 C -3.5 5.7 -2.5 4.6 -0.8 4.2 C 1 3.8 2.7 2.9 3.7 1.6 C 4.1 2.8 4.3 4.1 4.3 5.4 C 4.3 8.2 2.3 10.4 -0.3 10.4 Z";
    format!(
        r##"<g transform="translate({cx},{base_y})"><path d="{flame}" fill="{color}" fill-rule="evenodd"/></g>"##
    )
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
