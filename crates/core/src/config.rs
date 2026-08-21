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

#[derive(Debug, Clone, PartialEq, Eq)]
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
        })
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
