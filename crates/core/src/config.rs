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
pub struct GithubStatsConfig {
    pub username: String,
    pub token_env: String,
    pub cards: CardSelection,
    pub size: ImageSize,
    pub theme: String,
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
}
