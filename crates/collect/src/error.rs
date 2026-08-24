use std::{error::Error, fmt, path::PathBuf, time::Duration};

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
    /// The editor held its database's lock for longer than the collector was
    /// willing to wait. Separate from a schema error because the two ask for
    /// opposite responses: this one is waited out, that one is a code change.
    Busy {
        what: &'static str,
        waited: Duration,
    },
    /// A caller sent something the journal is not allowed to hold. Separate from
    /// the read errors because it is answerable: the daemon turns it into a 400.
    Rejected {
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
            Self::Busy { what, waited } => write!(
                formatter,
                "{what} stayed locked by its owner for more than {} seconds; \
                 the editor is mid-write, so try again rather than changing anything",
                waited.as_secs()
            ),
            Self::Rejected { message } => write!(formatter, "refused: {message}"),
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
