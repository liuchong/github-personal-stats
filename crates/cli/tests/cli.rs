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
            "--hide-language",
            "Ruby,HTML",
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
fn cli_configures_panel_content_and_refuses_a_list_it_cannot_draw() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures/github_user_data.json");
    let output = std::env::temp_dir().join(format!(
        "github-personal-stats-cli-{}-panels.svg",
        std::process::id()
    ));
    let status = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "generate",
            "--fixture",
            fixture.to_str().unwrap(),
            "--stat-rows",
            "reviews,repos",
            "--language-rows",
            "2",
            "--streak-sides",
            "active,current",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success());
    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(">Reviews<"));
    assert!(svg.contains(">Contributed To<"));
    assert!(svg.contains(">Active Days<"));
    assert!(!svg.contains(">Total Stars<"));
    assert!(!svg.contains(">Total Contributions<"));
    let _ = fs::remove_file(output);

    for (option, value, expected) in [
        ("--stat-rows", "stars,stars", "stars is listed twice"),
        ("--stat-rows", "starz", "unknown metric starz"),
        ("--streak-sides", "total", "expected two metrics"),
        ("--language-rows", "9", "expected a row count from 1 to 8"),
    ] {
        let refused = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
            .args([
                "generate",
                "--fixture",
                fixture.to_str().unwrap(),
                option,
                value,
            ])
            .output()
            .unwrap();

        assert!(!refused.status.success(), "{option} {value} should fail");
        assert!(
            String::from_utf8(refused.stderr)
                .unwrap()
                .contains(expected),
            "{option} {value} should explain: {expected}"
        );
    }
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
fn a_saved_profile_draws_every_tile_without_asking_github_anything() {
    let directory = std::env::temp_dir().join(format!("gps-saved-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let saved = directory.join("profile.json");
    fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../core/tests/fixtures/github_user_data.json"),
        &saved,
    )
    .unwrap();

    for (card, extra) in [
        ("stats", Vec::new()),
        ("languages", Vec::new()),
        ("heat", Vec::new()),
        ("metric", vec!["--metric", "total"]),
        ("metric", vec!["--metric", "longest"]),
    ] {
        let tile = directory.join(format!("{card}-{}.svg", extra.last().unwrap_or(&"only")));
        let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
            .args([
                "generate",
                "--card",
                card,
                "--fixture",
                saved.to_str().unwrap(),
                "--output",
                tile.to_str().unwrap(),
            ])
            .args(&extra)
            .env_remove("GITHUB_TOKEN")
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "{card} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(fs::read_to_string(&tile).unwrap().contains("<svg"));
    }

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn asking_a_saved_profile_to_be_fetched_differently_is_refused() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures/github_user_data.json");
    let target = std::env::temp_dir().join(format!("gps-refused-{}.svg", std::process::id()));

    for option in [
        vec!["--authored-languages"],
        vec!["--author-email", "old@example.com"],
        vec!["--min-repo-language-share", "1"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
            .args([
                "generate",
                "--card",
                "languages",
                "--fixture",
                fixture.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
            ])
            .args(&option)
            .output()
            .unwrap();

        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!output.status.success(), "{option:?} was accepted");
        assert!(stderr.contains(option[0]), "unhelpful message: {stderr}");
        assert!(stderr.contains("pass them to fetch instead"), "{stderr}");
    }

    // The one language option that acts after a fetch still works on saved data.
    let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "generate",
            "--card",
            "languages",
            "--fixture",
            fixture.to_str().unwrap(),
            "--hide-language",
            "Rust",
            "--output",
            target.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!fs::read_to_string(&target).unwrap().contains(">Rust<"));
    let _ = fs::remove_file(target);
}

#[test]
fn fetch_is_the_only_command_that_needs_the_network() {
    let saved = std::env::temp_dir().join(format!("gps-fetch-{}.json", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "fetch",
            "--user",
            "octo",
            "--output",
            saved.to_str().unwrap(),
        ])
        .env_remove("GITHUB_TOKEN")
        .output()
        .unwrap();

    // Reaching the token check is how we know the command was understood and
    // went looking for live data rather than being turned away as unknown.
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!output.status.success());
    assert!(
        stderr.contains("missing token environment variable GITHUB_TOKEN"),
        "unexpected failure: {stderr}"
    );
    assert!(!saved.exists(), "a failed fetch must not leave a file");
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
