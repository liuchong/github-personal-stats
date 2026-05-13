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
}

impl RenderTheme {
    pub fn named(name: &str) -> Self {
        match name {
            "dark" => Self {
                background: "#0d1117",
                panel: "#161b22",
                text: "#f0f6fc",
                muted: "#8b949e",
                accent: "#2f81f7",
                border: "#30363d",
            },
            "transparent" => Self {
                background: "transparent",
                panel: "transparent",
                text: "#24292f",
                muted: "#57606a",
                accent: "#0969da",
                border: "#d0d7de",
            },
            _ => Self {
                background: "#ffffff",
                panel: "#f6f8fa",
                text: "#24292f",
                muted: "#57606a",
                accent: "#0969da",
                border: "#d0d7de",
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
            panel(padding, padding, panel_width, top_height, "Stats", theme),
            stats_lines(stats, padding + 24, padding + 54, theme),
            panel(
                padding + panel_width + gap,
                padding,
                panel_width,
                top_height,
                "Languages",
                theme
            ),
            language_lines(
                languages,
                padding + panel_width + gap + 24,
                padding + 54,
                theme
            ),
            panel(padding, bottom_y, bottom_width, top_height, "Streak", theme),
            streak_lines(streak, padding + 24, bottom_y + 54, theme)
        ),
    )
}

fn render_stats_card(stats: &AggregatedStats, size: &ImageSize, theme: &RenderTheme) -> String {
    svg_root(
        size,
        theme,
        panel(16, 16, size.width - 32, size.height - 32, "Stats", theme)
            + &stats_lines(stats, 40, 70, theme),
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
            theme,
        ) + &language_lines(languages, 40, 70, theme),
    )
}

fn render_streak_card(streak: &StreakSummary, size: &ImageSize, theme: &RenderTheme) -> String {
    svg_root(
        size,
        theme,
        panel(16, 16, size.width - 32, size.height - 32, "Streak", theme)
            + &streak_lines(streak, 40, 70, theme),
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
            theme,
        ) + &wakatime_lines(summary, 40, 70, theme),
    )
}

fn render_status_card(state: &str, size: &ImageSize, theme: &RenderTheme) -> String {
    svg_root(
        size,
        theme,
        panel(16, 16, size.width - 32, size.height - 32, "Status", theme)
            + &text(40, 72, 16, theme.text, &escape_xml(state)),
    )
}

fn svg_root(size: &ImageSize, theme: &RenderTheme, body: String) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" role="img"><rect width="100%" height="100%" fill="{}"/>{}</svg>"#,
        size.width, size.height, size.width, size.height, theme.background, body
    )
}

fn panel(x: u32, y: u32, width: u32, height: u32, title: &str, theme: &RenderTheme) -> String {
    format!(
        r#"<g><rect x="{}" y="{}" width="{}" height="{}" rx="12" fill="{}" stroke="{}"/><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="18" font-weight="700" fill="{}">{}</text></g>"#,
        x,
        y,
        width,
        height,
        theme.panel,
        theme.border,
        x + 24,
        y + 32,
        theme.text,
        escape_xml(title)
    )
}

fn stats_lines(stats: &AggregatedStats, x: u32, y: u32, theme: &RenderTheme) -> String {
    [
        format!("Stars {}", stats.total_stars),
        format!("Commits {}", stats.total_commits),
        format!("Pull Requests {}", stats.total_pull_requests),
        format!("Issues {}", stats.total_issues),
        format!("Rank {}", stats.rank),
    ]
    .iter()
    .enumerate()
    .map(|(index, line)| text(x, y + index as u32 * 24, 14, theme.muted, line))
    .collect()
}

fn language_lines(languages: &[LanguageShare], x: u32, y: u32, theme: &RenderTheme) -> String {
    languages
        .iter()
        .take(5)
        .enumerate()
        .map(|(index, language)| {
            let percent = language.percentage_basis_points as f32 / 100.0;
            text(
                x,
                y + index as u32 * 24,
                14,
                theme.muted,
                &format!("{} {:.1}%", language.name, percent),
            )
        })
        .collect()
}

fn streak_lines(streak: &StreakSummary, x: u32, y: u32, theme: &RenderTheme) -> String {
    [
        format!("Current {}", streak.current),
        format!("Longest {}", streak.longest),
        format!("Active Days {}", streak.total_active_days),
    ]
    .iter()
    .enumerate()
    .map(|(index, line)| text(x, y + index as u32 * 24, 14, theme.muted, line))
    .collect()
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

fn progress_bar(seconds: u64, total: u64) -> String {
    let filled = if total == 0 { 0 } else { seconds * 10 / total };
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
