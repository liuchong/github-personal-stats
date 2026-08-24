use std::{error::Error, fmt, path::PathBuf};

use github_personal_stats_core::GithubStatsError;

#[derive(Debug)]
pub enum CollectError {
    NotFound {
        what: &'static str,
        path: PathBuf,
    },
    Unreadable {
        path: PathBuf,
        message: String,
    },
    UnexpectedSchema {
        source: &'static str,
        message: String,
    },
    Snapshot(GithubStatsError),
}

impl fmt::Display for CollectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { what, path } => {
                write!(formatter, "no {what} at {}", path.display())
            }
            Self::Unreadable { path, message } => {
                write!(formatter, "could not read {}: {message}", path.display())
            }
            Self::UnexpectedSchema { source, message } => write!(
                formatter,
                "{source} does not look the way this build expects: {message}"
            ),
            Self::Snapshot(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for CollectError {}

impl From<GithubStatsError> for CollectError {
    fn from(error: GithubStatsError) -> Self {
        Self::Snapshot(error)
    }
}
