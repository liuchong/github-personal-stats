use github_personal_stats_core::{OutputKind, parse_output_kind, workspace_info};

#[test]
fn workspace_info_exposes_dashboard_as_default() {
    let info = workspace_info();

    assert_eq!(info.name, "github-personal-stats");
    assert_eq!(info.default_output, OutputKind::Dashboard);
    assert!(info.supported_outputs.contains(&OutputKind::Dashboard));
    assert!(info.supported_outputs.contains(&OutputKind::WakatimeReadme));
}

#[test]
fn workspace_info_serializes_to_stable_json() {
    let json = workspace_info().to_json();

    assert!(json.contains(r#""name": "github-personal-stats""#));
    assert!(json.contains(r#""default_output": "dashboard""#));
    assert!(json.contains(r#""wakatime-readme""#));
}

#[test]
fn output_kind_parser_accepts_canonical_values() {
    assert_eq!(parse_output_kind("dashboard"), Ok(OutputKind::Dashboard));
    assert_eq!(parse_output_kind("stats"), Ok(OutputKind::Stats));
    assert_eq!(parse_output_kind("languages"), Ok(OutputKind::Languages));
    assert_eq!(parse_output_kind("streak"), Ok(OutputKind::Streak));
    assert_eq!(parse_output_kind("repo"), Ok(OutputKind::Repo));
    assert_eq!(parse_output_kind("gist"), Ok(OutputKind::Gist));
    assert_eq!(parse_output_kind("wakatime"), Ok(OutputKind::Wakatime));
    assert_eq!(
        parse_output_kind("wakatime-readme"),
        Ok(OutputKind::WakatimeReadme)
    );
    assert_eq!(parse_output_kind("status"), Ok(OutputKind::Status));
    assert_eq!(parse_output_kind("json"), Ok(OutputKind::Json));
    assert_eq!(parse_output_kind("png"), Ok(OutputKind::Png));
}

#[test]
fn output_kind_parser_accepts_aliases() {
    assert_eq!(
        parse_output_kind("top-languages"),
        Ok(OutputKind::Languages)
    );
    assert_eq!(parse_output_kind("top-langs"), Ok(OutputKind::Languages));
    assert_eq!(parse_output_kind("repository"), Ok(OutputKind::Repo));
    assert_eq!(
        parse_output_kind("coding-activity"),
        Ok(OutputKind::Wakatime)
    );
    assert_eq!(
        parse_output_kind("coding-activity-readme"),
        Ok(OutputKind::WakatimeReadme)
    );
}

#[test]
fn output_kind_parser_rejects_unknown_values() {
    let error = parse_output_kind("unknown-card").unwrap_err();

    assert_eq!(error.to_string(), "unsupported output kind: unknown-card");
}
