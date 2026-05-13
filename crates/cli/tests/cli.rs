use std::{fs, process::Command};

#[test]
fn cli_generates_dashboard_svg_file() {
    let output = std::env::temp_dir().join(format!(
        "github-personal-stats-cli-{}-dashboard.svg",
        std::process::id()
    ));
    let status = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "generate",
            "--card",
            "dashboard",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success());
    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"width="1000""#));
    assert!(svg.contains("Streak"));
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
        "before\n<!--START_SECTION:waka-->\nold\n<!--END_SECTION:waka-->\nafter\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "update-readme",
            "--target",
            target.to_str().unwrap(),
            "--section",
            "waka",
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
