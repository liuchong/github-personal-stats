pub mod activity;
pub mod activityblocks;
pub mod aggregation;
pub mod client;
pub mod color;
pub mod config;
pub mod data;
pub mod error;
pub mod json;
pub mod remote;
pub mod renderer;
pub mod store;
pub mod textchart;
pub mod workspace;

pub use activity::{
    ACTIVITY_SCHEMA, ActivitySnapshot, ActivityTotals, Author, AuthorShare, DayBucket, LineCounts,
    LineFact, LineShare, LineTotals, Lines, MEASURE_AGENT, MEASURE_EDITOR, MEASURE_IMPORTED,
    MeasureTotals, ModelUsage, TimeBucket, TimeFact, TimeTotals, TokenUsage, UNKNOWN_LANGUAGE,
    language_label, merge_snapshots, parse_activity_snapshot, summarise_activity,
    write_activity_snapshot,
};
pub use activityblocks::{
    BlockSpec, ChartRows, ChartValue, build_blocks, default_blocks, parse_blocks,
};
pub use aggregation::{
    ActivityComparison, ActivityLanguage, ActivityMeasure, ActivitySpan, ActivityWindow,
    AggregatedStats, CardData, CodingActivityEntry, CodingActivitySummary, LanguageShare,
    StreakMode, StreakSummary, aggregate_card_data, aggregate_coding_activity, aggregate_languages,
    aggregate_stats, calculate_streak, compare_activity,
};
pub use client::{GithubClient, GithubGraphqlRequest, MockGithubClient};
pub use color::{HEAT_RAMP_STEPS, HeatRamp, named_ramps};
pub use config::{
    CardSelection, DEFAULT_ACTIVITY_WINDOWS, DEFAULT_HEAT_THRESHOLD, DEFAULT_LANGUAGE_ROWS,
    GithubStatsConfig, HeatRing, HeatScale, HeatShape, HeatWindow, ImageSize, LanguageScope,
    MAX_ACTIVITY_WINDOW, MAX_LANGUAGE_ROWS, MAX_PADDING, MAX_SCALE_BASIS_POINTS,
    MIN_SCALE_BASIS_POINTS, StatMetric, StreakMetric, Theme, TileMetric,
};
pub use data::{ContributionDay, GithubData, GithubProfile, RepositoryLanguage, UserStats};
pub use error::{GithubStatsError, RemoteErrorKind};
pub use remote::GithubGraphqlClient;
pub use renderer::{
    RenderTheme, format_duration, format_duration_aligned, format_number, render_card,
    render_readme_section,
};
pub use textchart::{
    BarGlyphs, ChartBlock, ChartRow, ChartStyle, ChartSummary, Column, DEFAULT_BAR_CELLS,
    render_text_chart,
};
pub use workspace::{OutputKind, WorkspaceInfo, parse_output_kind, workspace_info};
