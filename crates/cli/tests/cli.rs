use std::{fs, process::Command};

#[test]
fn cli_generates_dashboard_svg_file() {
    let output = std::env::temp_dir().join(format!(
        "github-personal-stats-cli-{}-dashboard.svg",
        std::process::id()
    ));
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures/github_user_data.json");
    let status = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "generate",
            "--card",
            "dashboard",
            "--fixture",
            fixture.to_str().unwrap(),
            "--authored-languages",
            "--author-email",
            "old@example.com",
            "--hide-language",
            "Ruby,HTML",
            "--min-repo-language-share",
            "1",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success());
    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"width="1000""#));
    assert!(svg.contains("Streak"));
    assert!(!svg.contains("HTML"));
    let _ = fs::remove_file(output);
}

#[test]
fn cli_updates_marked_readme_section() {
    let target = std::env::temp_dir().join(format!(
        "github-personal-stats-cli-{}-README.md",
        std::process::id()
    ));
    fs::write(
        &target,
        "before\n<!--START_SECTION:activity-->\nold\n<!--END_SECTION:activity-->\nafter\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "update-readme",
            "--target",
            target.to_str().unwrap(),
            "--section",
            "activity",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    let readme = fs::read_to_string(&target).unwrap();
    assert!(readme.contains("before"));
    assert!(readme.contains("### Coding Activity"));
    assert!(readme.contains("after"));
    assert!(!readme.contains("old"));
    let _ = fs::remove_file(target);
}

#[test]
fn cli_defaults_to_workspace_info() {
    let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#""name": "github-personal-stats""#));
    assert!(stdout.contains(r#""default_output": "dashboard""#));
}

#[test]
fn cli_reports_unsupported_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .arg("unknown")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unsupported command: unknown"));
}

#[test]
fn cli_reports_missing_readme_section_marker() {
    let target = std::env::temp_dir().join(format!(
        "github-personal-stats-cli-{}-missing-section.md",
        std::process::id()
    ));
    fs::write(&target, "no generated section here\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args(["update-readme", "--target", target.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("missing section marker: <!--START_SECTION:activity-->"));
    let _ = fs::remove_file(target);
}
