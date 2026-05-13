use crate::{GithubStatsError, OutputKind, parse_output_kind};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubStatsConfig {
    pub username: String,
    pub token_env: String,
    pub cards: CardSelection,
    pub size: ImageSize,
    pub theme: String,
    pub language_scope: LanguageScope,
    pub author_emails: Vec<String>,
    pub hidden_languages: Vec<String>,
    pub min_repo_language_share_basis_points: u32,
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
            theme: "default".to_owned(),
            language_scope: LanguageScope::Owned,
            author_emails: Vec::new(),
            hidden_languages: Vec::new(),
            min_repo_language_share_basis_points: 0,
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
}
