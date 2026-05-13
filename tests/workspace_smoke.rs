use github_stats_core::{OutputKind, workspace_info};

#[test]
fn workspace_default_output_is_dashboard() {
    assert_eq!(workspace_info().default_output, OutputKind::Dashboard);
}
