use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    Dashboard,
    Stats,
    Languages,
    Streak,
    Repo,
    Gist,
    Wakatime,
    WakatimeReadme,
    Status,
    Json,
    Png,
}

impl OutputKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dashboard => "dashboard",
            Self::Stats => "stats",
            Self::Languages => "languages",
            Self::Streak => "streak",
            Self::Repo => "repo",
            Self::Gist => "gist",
            Self::Wakatime => "wakatime",
            Self::WakatimeReadme => "wakatime-readme",
            Self::Status => "status",
            Self::Json => "json",
            Self::Png => "png",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub default_output: OutputKind,
    pub supported_outputs: Vec<OutputKind>,
}

impl WorkspaceInfo {
    pub fn to_json(&self) -> String {
        let supported_outputs = self
            .supported_outputs
            .iter()
            .map(|output| format!("\"{}\"", output.as_str()))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "{{\n  \"name\": \"{}\",\n  \"version\": \"{}\",\n  \"default_output\": \"{}\",\n  \"supported_outputs\": [{}]\n}}",
            self.name,
            self.version,
            self.default_output.as_str(),
            supported_outputs
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum GithubStatsError {
    UnsupportedOutputKind { value: String },
}

impl fmt::Display for GithubStatsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOutputKind { value } => {
                write!(formatter, "unsupported output kind: {value}")
            }
        }
    }
}

impl Error for GithubStatsError {}

pub fn workspace_info() -> WorkspaceInfo {
    WorkspaceInfo {
        name: "github-stats",
        version: env!("CARGO_PKG_VERSION"),
        default_output: OutputKind::Dashboard,
        supported_outputs: vec![
            OutputKind::Dashboard,
            OutputKind::Stats,
            OutputKind::Languages,
            OutputKind::Streak,
            OutputKind::Repo,
            OutputKind::Gist,
            OutputKind::Wakatime,
            OutputKind::WakatimeReadme,
            OutputKind::Status,
            OutputKind::Json,
            OutputKind::Png,
        ],
    }
}

pub fn parse_output_kind(value: &str) -> Result<OutputKind, GithubStatsError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "dashboard" => Ok(OutputKind::Dashboard),
        "stats" => Ok(OutputKind::Stats),
        "languages" | "top-languages" | "top-langs" => Ok(OutputKind::Languages),
        "streak" => Ok(OutputKind::Streak),
        "repo" | "repository" => Ok(OutputKind::Repo),
        "gist" => Ok(OutputKind::Gist),
        "wakatime" | "coding-activity" => Ok(OutputKind::Wakatime),
        "wakatime-readme" | "coding-activity-readme" => Ok(OutputKind::WakatimeReadme),
        "status" => Ok(OutputKind::Status),
        "json" => Ok(OutputKind::Json),
        "png" => Ok(OutputKind::Png),
        _ => Err(GithubStatsError::UnsupportedOutputKind {
            value: value.to_owned(),
        }),
    }
}
