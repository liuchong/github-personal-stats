use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteErrorKind {
    Authentication,
    Permission,
    NotFound,
    RateLimit,
    UpstreamUnavailable,
    InvalidResponse,
    UnsupportedConfiguration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubStatsError {
    UnsupportedOutputKind {
        value: String,
    },
    InvalidConfig {
        field: &'static str,
        message: String,
    },
    Remote {
        kind: RemoteErrorKind,
        message: String,
    },
    InvalidResponse {
        message: String,
    },
}

impl fmt::Display for GithubStatsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOutputKind { value } => {
                write!(formatter, "unsupported output kind: {value}")
            }
            Self::InvalidConfig { field, message } => {
                write!(formatter, "invalid config for {field}: {message}")
            }
            Self::Remote { kind, message } => {
                write!(formatter, "remote error {kind:?}: {message}")
            }
            Self::InvalidResponse { message } => write!(formatter, "invalid response: {message}"),
        }
    }
}

impl Error for GithubStatsError {}
