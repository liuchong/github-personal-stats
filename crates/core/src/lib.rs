pub mod aggregation;
pub mod client;
pub mod config;
pub mod data;
pub mod error;
pub mod json;
pub mod renderer;
pub mod workspace;

pub use aggregation::{
    AggregatedStats, CardData, CodingActivityEntry, CodingActivitySummary, LanguageShare,
    StreakMode, StreakSummary, aggregate_card_data, aggregate_coding_activity, aggregate_languages,
    aggregate_stats, calculate_streak,
};
pub use client::{GithubClient, GithubGraphqlClient, GithubGraphqlRequest, MockGithubClient};
pub use config::{CardSelection, GithubStatsConfig, ImageSize};
pub use data::{ContributionDay, GithubData, GithubProfile, RepositoryLanguage, UserStats};
pub use error::{GithubStatsError, RemoteErrorKind};
pub use renderer::{RenderTheme, render_card, render_readme_section};
pub use workspace::{OutputKind, WorkspaceInfo, parse_output_kind, workspace_info};
