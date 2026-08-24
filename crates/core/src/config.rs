use crate::{GithubStatsError, HeatRamp, OutputKind, parse_output_kind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

impl ImageSize {
    pub fn new(width: u32, height: u32) -> Result<Self, GithubStatsError> {
        if width == 0 || height == 0 {
            return Err(GithubStatsError::InvalidConfig {
                field: "size",
                message: "width and height must be positive".to_owned(),
            });
        }

        Ok(Self { width, height })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardSelection {
    pub outputs: Vec<OutputKind>,
}

impl CardSelection {
    pub fn parse(value: &str) -> Result<Self, GithubStatsError> {
        let outputs = value
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(parse_output_kind)
            .collect::<Result<Vec<_>, _>>()?;

        if outputs.is_empty() {
            return Err(GithubStatsError::InvalidConfig {
                field: "card",
                message: "at least one card is required".to_owned(),
            });
        }

        Ok(Self { outputs })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageScope {
    Owned,
    Authored,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Light,
    Dark,
    Transparent,
}

impl Theme {
    pub fn parse(value: &str) -> Result<Self, GithubStatsError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "light" | "default" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            "transparent" => Ok(Self::Transparent),
            _ => Err(GithubStatsError::InvalidConfig {
                field: "theme",
                message: "expected light, dark, or transparent".to_owned(),
            }),
        }
    }
}

pub const DEFAULT_HEAT_THRESHOLD: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeatWindow {
    Streak,
    Fixed(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeatShape {
    Segmented,
    Ticks,
    Arcs,
    Bands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeatScale {
    Linear,
    Sqrt,
    Log,
    Quantile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeatRing {
    pub window: HeatWindow,
    pub limit: Option<u32>,
    pub shape: HeatShape,
    pub threshold: u32,
    pub scale: HeatScale,
    pub ramp: HeatRamp,
    pub label: Option<String>,
}

impl HeatRing {
    pub fn span(&self, streak: u32) -> u32 {
        let span = match self.window {
            HeatWindow::Streak => streak,
            HeatWindow::Fixed(days) => days,
        };
        self.limit.map_or(span, |limit| span.min(limit))
    }

    pub fn label_template(&self) -> &str {
        self.label.as_deref().unwrap_or(match self.window {
            HeatWindow::Streak => "{Y}",
            HeatWindow::Fixed(_) => "{X}/last {Y}",
        })
    }
}

impl Default for HeatRing {
    fn default() -> Self {
        Self {
            window: HeatWindow::Streak,
            limit: None,
            shape: HeatShape::Segmented,
            threshold: DEFAULT_HEAT_THRESHOLD,
            scale: HeatScale::Linear,
            ramp: HeatRamp::default(),
            label: None,
        }
    }
}

/// A row the stats panel can list. `AggregatedStats` carries all six, and every
/// one of them already feeds the rank score, so any of them can be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatMetric {
    Stars,
    Commits,
    PullRequests,
    Issues,
    Reviews,
    ContributedTo,
}

impl StatMetric {
    pub fn parse(value: &str) -> Result<Self, GithubStatsError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "stars" => Ok(Self::Stars),
            "commits" => Ok(Self::Commits),
            "prs" | "pull-requests" => Ok(Self::PullRequests),
            "issues" => Ok(Self::Issues),
            "reviews" => Ok(Self::Reviews),
            "repos" | "contributed" => Ok(Self::ContributedTo),
            other => Err(GithubStatsError::InvalidConfig {
                field: "stat_rows",
                message: format!(
                    "unknown metric {other}; expected stars, commits, prs, issues, reviews, or repos"
                ),
            }),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Stars => "stars",
            Self::Commits => "commits",
            Self::PullRequests => "prs",
            Self::Issues => "issues",
            Self::Reviews => "reviews",
            Self::ContributedTo => "repos",
        }
    }
}

/// A figure one of the streak panels can report beside the ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreakMetric {
    TotalContributions,
    LongestStreak,
    CurrentStreak,
    ActiveDays,
}

impl StreakMetric {
    pub fn parse(value: &str) -> Result<Self, GithubStatsError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "total" => Ok(Self::TotalContributions),
            "longest" => Ok(Self::LongestStreak),
            "current" => Ok(Self::CurrentStreak),
            "active" => Ok(Self::ActiveDays),
            other => Err(GithubStatsError::InvalidConfig {
                field: "streak_sides",
                message: format!(
                    "unknown metric {other}; expected total, longest, current, or active"
                ),
            }),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::TotalContributions => "total",
            Self::LongestStreak => "longest",
            Self::CurrentStreak => "current",
            Self::ActiveDays => "active",
        }
    }
}

/// Any single figure the card set can report, drawn on its own so a README can
/// place it wherever it likes. Reuses the panel vocabularies rather than a third
/// list of names, so `stars` means the same thing on a tile as in a stats row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileMetric {
    Stat(StatMetric),
    Streak(StreakMetric),
}

impl TileMetric {
    pub fn parse(value: &str) -> Result<Self, GithubStatsError> {
        if let Ok(stat) = StatMetric::parse(value) {
            return Ok(Self::Stat(stat));
        }

        StreakMetric::parse(value)
            .map(Self::Streak)
            .map_err(|_| GithubStatsError::InvalidConfig {
                field: "metric",
                message: format!(
                    "unknown metric {}; expected stars, commits, prs, issues, reviews, repos, \
                     total, longest, current, or active",
                    value.trim()
                ),
            })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Stat(metric) => metric.name(),
            Self::Streak(metric) => metric.name(),
        }
    }
}

impl Default for TileMetric {
    fn default() -> Self {
        Self::Streak(StreakMetric::CurrentStreak)
    }
}

/// Both metric lists are ordered and must not repeat a metric: a panel showing
/// the same figure twice is a typo, not a layout choice, so it is refused rather
/// than silently collapsed.
fn parse_metrics<T: Copy + PartialEq>(
    field: &'static str,
    value: &str,
    parse_one: fn(&str) -> Result<T, GithubStatsError>,
    name: fn(T) -> &'static str,
) -> Result<Vec<T>, GithubStatsError> {
    let mut metrics = Vec::new();

    for part in value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let metric = parse_one(part)?;
        if metrics.contains(&metric) {
            return Err(GithubStatsError::InvalidConfig {
                field,
                message: format!("{} is listed twice", name(metric)),
            });
        }
        metrics.push(metric);
    }

    if metrics.is_empty() {
        return Err(GithubStatsError::InvalidConfig {
            field,
            message: "expected a comma separated list of metrics".to_owned(),
        });
    }

    Ok(metrics)
}

/// Scaling further than this either shrinks the text past reading size or blows
/// the layout up past anything a README column can hold.
pub const MIN_SCALE_BASIS_POINTS: u32 = 5_000;
pub const MAX_SCALE_BASIS_POINTS: u32 = 40_000;

/// Beyond this a card has more margin than content on any tile worth drawing.
pub const MAX_PADDING: u32 = 64;

pub const DEFAULT_LANGUAGE_ROWS: usize = 6;

/// Aggregation keeps the top eight languages, so the panel cannot promise more
/// than that, and a taller list would not fit the dashboard column anyway.
pub const MAX_LANGUAGE_ROWS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GithubStatsConfig {
    pub username: String,
    pub token_env: String,
    pub cards: CardSelection,
    pub size: ImageSize,
    pub theme: Theme,
    pub language_scope: LanguageScope,
    pub author_emails: Vec<String>,
    pub hidden_languages: Vec<String>,
    pub min_repo_language_share_basis_points: u32,
    pub heat_ring: HeatRing,
    pub stat_rows: Vec<StatMetric>,
    pub language_rows: usize,
    pub streak_sides: [StreakMetric; 2],
    /// Which figure a single-metric tile reports. Ignored by every other card.
    pub metric: TileMetric,
    /// Inner margin in pixels. `None` scales it with the card width, which is
    /// right for a card seen alone; pinning it keeps the content edges of tiles
    /// of different widths in line when they are composed into one block.
    pub padding: Option<u32>,
    /// Fit the card to its content instead of to a height chosen up front.
    /// Cards that divide a height between sections do not support this.
    pub auto_height: bool,
    /// Multiplier, in basis points, between the size the card is laid out at and
    /// the size it is displayed at. The drawing is vector, so this only changes
    /// how large everything appears at a given display width.
    pub scale_basis_points: u32,
}

impl GithubStatsConfig {
    pub fn new(username: impl Into<String>) -> Result<Self, GithubStatsError> {
        let username = username.into();
        if username.trim().is_empty() {
            return Err(GithubStatsError::InvalidConfig {
                field: "username",
                message: "username is required".to_owned(),
            });
        }

        Ok(Self {
            username,
            token_env: "GITHUB_TOKEN".to_owned(),
            cards: CardSelection {
                outputs: vec![OutputKind::Dashboard],
            },
            size: ImageSize {
                width: 1000,
                height: 420,
            },
            theme: Theme::Light,
            language_scope: LanguageScope::Owned,
            author_emails: Vec::new(),
            hidden_languages: Vec::new(),
            min_repo_language_share_basis_points: 0,
            heat_ring: HeatRing::default(),
            stat_rows: vec![
                StatMetric::Stars,
                StatMetric::Commits,
                StatMetric::PullRequests,
                StatMetric::Issues,
            ],
            language_rows: DEFAULT_LANGUAGE_ROWS,
            streak_sides: [
                StreakMetric::TotalContributions,
                StreakMetric::LongestStreak,
            ],
            metric: TileMetric::default(),
            padding: None,
            auto_height: false,
            scale_basis_points: 10_000,
        })
    }

    pub fn with_scale(mut self, value: &str) -> Result<Self, GithubStatsError> {
        let text = value.trim();
        let scale = text
            .parse::<f64>()
            .ok()
            .filter(|scale| scale.is_finite())
            .map(|scale| (scale * 10_000.0).round())
            .filter(|points| *points >= 0.0 && *points <= f64::from(u32::MAX))
            .map(|points| points as u32)
            .ok_or_else(|| GithubStatsError::InvalidConfig {
                field: "scale",
                message: format!("expected a multiplier such as 1.5, got {text}"),
            })?;

        if !(MIN_SCALE_BASIS_POINTS..=MAX_SCALE_BASIS_POINTS).contains(&scale) {
            return Err(GithubStatsError::InvalidConfig {
                field: "scale",
                message: format!(
                    "{text} is outside {} to {}",
                    f64::from(MIN_SCALE_BASIS_POINTS) / 10_000.0,
                    f64::from(MAX_SCALE_BASIS_POINTS) / 10_000.0
                ),
            });
        }

        self.scale_basis_points = scale;
        Ok(self)
    }

    pub fn with_auto_height(mut self) -> Self {
        self.auto_height = true;
        self
    }

    pub fn with_padding(mut self, value: &str) -> Result<Self, GithubStatsError> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("auto") {
            self.padding = None;
            return Ok(self);
        }

        let pixels = value
            .parse::<u32>()
            .map_err(|_| GithubStatsError::InvalidConfig {
                field: "padding",
                message: format!("expected a pixel count or auto, got {value}"),
            })?;

        if pixels > MAX_PADDING {
            return Err(GithubStatsError::InvalidConfig {
                field: "padding",
                message: format!("{pixels}px leaves too little room; the most is {MAX_PADDING}"),
            });
        }

        self.padding = Some(pixels);
        Ok(self)
    }

    pub fn with_metric(mut self, value: &str) -> Result<Self, GithubStatsError> {
        self.metric = TileMetric::parse(value)?;
        Ok(self)
    }

    pub fn with_stat_rows(mut self, value: &str) -> Result<Self, GithubStatsError> {
        self.stat_rows = parse_metrics("stat_rows", value, StatMetric::parse, StatMetric::name)?;
        Ok(self)
    }

    pub fn with_language_rows(mut self, value: &str) -> Result<Self, GithubStatsError> {
        let invalid = || GithubStatsError::InvalidConfig {
            field: "language_rows",
            message: format!("expected a row count from 1 to {MAX_LANGUAGE_ROWS}, got {value}"),
        };
        let rows = value.trim().parse::<usize>().map_err(|_| invalid())?;
        if rows == 0 || rows > MAX_LANGUAGE_ROWS {
            return Err(invalid());
        }

        self.language_rows = rows;
        Ok(self)
    }

    pub fn with_streak_sides(mut self, value: &str) -> Result<Self, GithubStatsError> {
        let sides = parse_metrics(
            "streak_sides",
            value,
            StreakMetric::parse,
            StreakMetric::name,
        )?;
        let [left, right] = sides.as_slice() else {
            return Err(GithubStatsError::InvalidConfig {
                field: "streak_sides",
                message: format!(
                    "expected two metrics for the left and right panel, got {}",
                    sides.len()
                ),
            });
        };

        self.streak_sides = [*left, *right];
        Ok(self)
    }

    pub fn with_cards(mut self, value: &str) -> Result<Self, GithubStatsError> {
        self.cards = CardSelection::parse(value)?;
        Ok(self)
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Result<Self, GithubStatsError> {
        self.size = ImageSize::new(width, height)?;
        Ok(self)
    }

    pub fn with_theme(mut self, value: &str) -> Result<Self, GithubStatsError> {
        self.theme = Theme::parse(value)?;
        Ok(self)
    }

    pub fn with_authored_languages(mut self) -> Self {
        self.language_scope = LanguageScope::Authored;
        self
    }

    pub fn with_author_emails(mut self, emails: Vec<String>) -> Self {
        self.author_emails = emails
            .into_iter()
            .flat_map(|email| {
                email
                    .split(',')
                    .map(|part| part.trim().to_owned())
                    .collect::<Vec<_>>()
            })
            .filter(|email| !email.is_empty())
            .collect();
        self
    }

    pub fn with_hidden_languages(mut self, languages: Vec<String>) -> Self {
        self.hidden_languages = languages
            .into_iter()
            .flat_map(|language| {
                language
                    .split(',')
                    .map(|part| part.trim().to_owned())
                    .collect::<Vec<_>>()
            })
            .filter(|language| !language.is_empty())
            .collect();
        self
    }

    pub fn with_min_repo_language_share(mut self, value: &str) -> Result<Self, GithubStatsError> {
        let percentage = value
            .parse::<f64>()
            .map_err(|_| GithubStatsError::InvalidConfig {
                field: "min_repo_language_share",
                message: "must be a percentage between 0 and 100".to_owned(),
            })?;
        if !(0.0..=100.0).contains(&percentage) || !percentage.is_finite() {
            return Err(GithubStatsError::InvalidConfig {
                field: "min_repo_language_share",
                message: "must be a percentage between 0 and 100".to_owned(),
            });
        }
        self.min_repo_language_share_basis_points = (percentage * 100.0).round() as u32;
        Ok(self)
    }

    pub fn with_heat_window(mut self, value: &str) -> Result<Self, GithubStatsError> {
        let trimmed = value.trim();
        self.heat_ring.window = if trimmed.eq_ignore_ascii_case("streak") {
            HeatWindow::Streak
        } else {
            HeatWindow::Fixed(positive_days("heat_window", trimmed)?)
        };
        Ok(self)
    }

    pub fn with_heat_limit(mut self, value: &str) -> Result<Self, GithubStatsError> {
        let trimmed = value.trim();
        self.heat_ring.limit = if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
            None
        } else {
            Some(positive_days("heat_limit", trimmed)?)
        };
        Ok(self)
    }

    pub fn with_heat_shape(mut self, value: &str) -> Result<Self, GithubStatsError> {
        self.heat_ring.shape = match value.trim().to_ascii_lowercase().as_str() {
            "segmented" => HeatShape::Segmented,
            "ticks" => HeatShape::Ticks,
            "arcs" => HeatShape::Arcs,
            "bands" => HeatShape::Bands,
            _ => {
                return Err(GithubStatsError::InvalidConfig {
                    field: "heat_shape",
                    message: "expected segmented, ticks, arcs, or bands".to_owned(),
                });
            }
        };
        Ok(self)
    }

    pub fn with_heat_threshold(mut self, value: &str) -> Result<Self, GithubStatsError> {
        self.heat_ring.threshold = positive_days("heat_threshold", value.trim())?;
        Ok(self)
    }

    pub fn with_heat_scale(mut self, value: &str) -> Result<Self, GithubStatsError> {
        self.heat_ring.scale = match value.trim().to_ascii_lowercase().as_str() {
            "linear" => HeatScale::Linear,
            "sqrt" => HeatScale::Sqrt,
            "log" => HeatScale::Log,
            "quantile" => HeatScale::Quantile,
            _ => {
                return Err(GithubStatsError::InvalidConfig {
                    field: "heat_scale",
                    message: "expected linear, sqrt, log, or quantile".to_owned(),
                });
            }
        };
        Ok(self)
    }

    pub fn with_heat_color(mut self, value: &str) -> Result<Self, GithubStatsError> {
        self.heat_ring.ramp = HeatRamp::parse(value)?;
        Ok(self)
    }

    pub fn with_heat_label(mut self, value: &str) -> Self {
        let trimmed = value.trim();
        self.heat_ring.label = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        };
        self
    }
}

fn positive_days(field: &'static str, value: &str) -> Result<u32, GithubStatsError> {
    match value.parse::<u32>() {
        Ok(days) if days > 0 => Ok(days),
        _ => Err(GithubStatsError::InvalidConfig {
            field,
            message: "must be a whole number of days above zero".to_owned(),
        }),
    }
}
