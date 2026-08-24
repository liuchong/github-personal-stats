pub mod aggregation;
pub mod client;
pub mod color;
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
pub use color::{HEAT_RAMP_STEPS, HeatRamp, named_ramps};
pub use config::{
    CardSelection, DEFAULT_HEAT_THRESHOLD, DEFAULT_LANGUAGE_ROWS, GithubStatsConfig, HeatRing,
    HeatScale, HeatShape, HeatWindow, ImageSize, LanguageScope, MAX_LANGUAGE_ROWS, MAX_PADDING,
    StatMetric, StreakMetric, Theme, TileMetric,
};
pub use data::{ContributionDay, GithubData, GithubProfile, RepositoryLanguage, UserStats};
pub use error::{GithubStatsError, RemoteErrorKind};
pub use renderer::{RenderTheme, render_card, render_readme_section};
pub use workspace::{OutputKind, WorkspaceInfo, parse_output_kind, workspace_info};
