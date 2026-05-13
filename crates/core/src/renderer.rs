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
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" role="img"><defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="{}"/><stop offset="100%" stop-color="{}"/></linearGradient><filter id="shadow" x="-10%" y="-10%" width="120%" height="130%"><feDropShadow dx="0" dy="8" stdDeviation="10" flood-color="#1f2937" flood-opacity="0.12"/></filter></defs><rect width="100%" height="100%" fill="url(#bg)"/>{}</svg>"##,
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
        r#"<g filter="url(#shadow)"><rect x="{}" y="{}" width="{}" height="{}" rx="18" fill="{}" stroke="{}"/><rect x="{}" y="{}" width="5" height="34" rx="2.5" fill="{}"/><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="19" font-weight="700" fill="{}">{}</text><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="12" fill="{}">{}</text></g>"#,
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
    let tile_width = (width - 12) / 2;
    [
        metric_tile(
            x,
            y,
            tile_width,
            "Stars",
            stats.total_stars,
            theme.accent,
            theme,
        ),
        metric_tile(
            x + tile_width + 12,
            y,
            tile_width,
            "Commits",
            stats.total_commits,
            theme.success,
            theme,
        ),
        metric_tile(
            x,
            y + 58,
            tile_width,
            "Pull Requests",
            stats.total_pull_requests,
            "#fb8500",
            theme,
        ),
        metric_tile(
            x + tile_width + 12,
            y + 58,
            tile_width,
            "Rank",
            stats.rank,
            "#d63384",
            theme,
        ),
    ]
    .join("")
}

fn metric_tile<T: ToString>(
    x: u32,
    y: u32,
    width: u32,
    label: &str,
    value: T,
    accent: &str,
    theme: &RenderTheme,
) -> String {
    format!(
        r#"<g><rect x="{}" y="{}" width="{}" height="46" rx="12" fill="{}" stroke="{}"/><circle cx="{}" cy="{}" r="5" fill="{}"/><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="11" fill="{}">{}</text><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="18" font-weight="700" fill="{}">{}</text></g>"#,
        x,
        y,
        width,
        theme.accent_soft,
        theme.border,
        x + 18,
        y + 17,
        accent,
        x + 32,
        y + 19,
        theme.muted,
        escape_xml(label),
        x + 16,
        y + 37,
        theme.text,
        escape_xml(&value.to_string())
    )
}

fn language_bars(
    languages: &[LanguageShare],
    x: u32,
    y: u32,
    width: u32,
    theme: &RenderTheme,
) -> String {
    languages
        .iter()
        .take(5)
        .enumerate()
        .map(|(index, language)| {
            let row_y = y + index as u32 * 24;
            let percentage = language.percentage_basis_points as f32 / 100.0;
            let bar_width = width.saturating_sub(150);
            let filled = bar_width * language.percentage_basis_points / 10_000;
            format!(
                r#"{}{}{}{}"#,
                text(x, row_y + 10, 12, theme.text, &language.name),
                text(
                    x + width - 58,
                    row_y + 10,
                    12,
                    theme.muted,
                    &format!("{percentage:.1}%")
                ),
                rounded_rect(x + 94, row_y, bar_width, 10, 5, theme.accent_soft, "none"),
                rounded_rect(
                    x + 94,
                    row_y,
                    filled,
                    10,
                    5,
                    language_color(&language.name, index),
                    "none"
                )
            )
        })
        .collect()
}

fn streak_tiles(streak: &StreakSummary, x: u32, y: u32, width: u32, theme: &RenderTheme) -> String {
    let tile_width = (width - 24) / 3;
    [
        large_metric(
            x,
            y,
            tile_width,
            "Current",
            streak.current,
            theme.accent,
            theme,
        ),
        large_metric(
            x + tile_width + 12,
            y,
            tile_width,
            "Longest",
            streak.longest,
            theme.success,
            theme,
        ),
        large_metric(
            x + (tile_width + 12) * 2,
            y,
            tile_width,
            "Active Days",
            streak.total_active_days,
            "#fb8500",
            theme,
        ),
    ]
    .join("")
}

fn large_metric<T: ToString>(
    x: u32,
    y: u32,
    width: u32,
    label: &str,
    value: T,
    accent: &str,
    theme: &RenderTheme,
) -> String {
    format!(
        r#"<g><rect x="{}" y="{}" width="{}" height="78" rx="16" fill="{}" stroke="{}"/><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="12" fill="{}">{}</text><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="34" font-weight="800" fill="{}">{}</text><rect x="{}" y="{}" width="{}" height="4" rx="2" fill="{}"/></g>"#,
        x,
        y,
        width,
        theme.accent_soft,
        theme.border,
        x + 18,
        y + 24,
        theme.muted,
        escape_xml(label),
        x + 18,
        y + 58,
        theme.text,
        escape_xml(&value.to_string()),
        x + 18,
        y + 66,
        width.saturating_sub(36),
        accent
    )
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
        r#"<g><rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}"/><text x="{}" y="{}" font-family="Arial, sans-serif" font-size="15" font-weight="700" fill="{}">{}</text></g>"#,
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
