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
fn cli_renders_each_theme_and_refuses_an_unknown_one() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures/github_user_data.json");
    for (theme, expected) in [("light", "#ffffff"), ("dark", "#0d1117")] {
        let output = std::env::temp_dir().join(format!(
            "github-personal-stats-cli-{}-{theme}.svg",
            std::process::id()
        ));
        let status = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
            .args([
                "generate",
                "--fixture",
                fixture.to_str().unwrap(),
                "--theme",
                theme,
                "--output",
                output.to_str().unwrap(),
            ])
            .status()
            .unwrap();

        assert!(status.success());
        let svg = fs::read_to_string(&output).unwrap();
        assert!(
            svg.contains(&format!(r#"fill="{expected}""#)),
            "the {theme} theme must paint the card background {expected}"
        );
        let _ = fs::remove_file(output);
    }

    let refused = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "generate",
            "--fixture",
            fixture.to_str().unwrap(),
            "--theme",
            "drak",
        ])
        .output()
        .unwrap();

    assert!(!refused.status.success());
    assert!(
        String::from_utf8(refused.stderr)
            .unwrap()
            .contains("expected light, dark, or transparent")
    );
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

#[test]
fn cli_reports_missing_token_for_live_generation() {
    let target = std::env::temp_dir().join(format!(
        "github-personal-stats-cli-{}-live.svg",
        std::process::id()
    ));

    let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .env_remove("GITHUB_TOKEN")
        .args(["generate", "--output", target.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("missing token environment variable GITHUB_TOKEN"));
    assert!(!target.exists());
}

#[test]
fn cli_prints_usage_for_every_help_spelling() {
    for arguments in [
        vec!["help"],
        vec!["--help"],
        vec!["-h"],
        vec!["generate", "--help"],
        vec!["update-readme", "-h"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
            .args(&arguments)
            .output()
            .unwrap();

        assert!(output.status.success(), "{arguments:?} should succeed");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.starts_with("github-personal-stats <command> [options]"),
            "{arguments:?} printed {stdout}"
        );
    }
}

#[test]
fn cli_help_asking_does_not_render_or_rewrite_anything() {
    let target = std::env::temp_dir().join(format!(
        "github-personal-stats-cli-{}-help.svg",
        std::process::id()
    ));

    let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .env_remove("GITHUB_TOKEN")
        .args(["generate", "--output", target.to_str().unwrap(), "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!target.exists(), "help must not write a card");
}

#[test]
fn cli_help_documents_every_option_it_parses() {
    let source = include_str!("../src/main.rs");
    let help = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .arg("help")
        .output()
        .unwrap();
    let help = String::from_utf8(help.stdout).unwrap();

    let mut parsed = source
        .split('"')
        .filter(|token| token.starts_with("--") && token.len() > 2)
        .collect::<Vec<_>>();
    parsed.sort_unstable();
    parsed.dedup();

    assert!(parsed.len() > 15, "found only {parsed:?}");
    for option in parsed {
        assert!(help.contains(option), "help is missing {option}");
    }
}

#[test]
fn cli_help_shows_the_defaults_the_code_uses() {
    let help = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .arg("help")
        .output()
        .unwrap();
    let help = String::from_utf8(help.stdout).unwrap();

    for expected in [
        "default: octo",
        "default: dashboard",
        "default: profile/github-personal-stats.svg",
        "default: 1000",
        "default: 420",
        "default: README.md",
        "default: activity",
    ] {
        assert!(help.contains(expected), "help is missing {expected}");
    }
}

#[test]
fn cli_points_an_unsupported_command_at_the_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .arg("render-everything")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unsupported command: render-everything"));
    assert!(stderr.contains("Commands:"));
}
