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

/// A record with one day in it, laid out the way a collector leaves one: a
/// directory per machine holding dated day files.
///
/// A root of its own for each call. Tests run at the same time and each removes
/// the record it made, so a shared path would have one deleting the record
/// another was still reading.
fn a_record_with_one_day() -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let root = std::env::temp_dir().join(format!(
        "gps-record-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let machine = root.join("m-test");
    fs::create_dir_all(&machine).unwrap();
    fs::write(
        machine.join("2026-08-20.json"),
        r#"{"schema":2,"machine":"m-test","date":"2026-08-20","time":{"agent":{"seconds":3600,"languages":{"Rust":3600},"sessions":1}},"lines":[{"language":"Rust","author":"agent","model":"a-model","added":500}]}"#,
    )
    .unwrap();
    root
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

    let record = a_record_with_one_day();
    let status = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "update-readme",
            "--target",
            target.to_str().unwrap(),
            "--section",
            "activity",
            "--activity-record",
            record.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success());
    let readme = fs::read_to_string(&target).unwrap();
    assert!(readme.contains("before"));
    // What lands in the README is the chart the record describes, in a fenced
    // block so the columns keep their alignment wherever it is read. Named as
    // `text` and not `txt`, which GitHub takes for a language and colours.
    assert!(readme.contains("```text\n"), "{readme}");
    assert!(!readme.contains("```txt\n"), "{readme}");
    assert!(readme.contains("LINES BY LANGUAGE"), "{readme}");
    assert!(readme.contains("Rust"), "{readme}");
    let _ = fs::remove_dir_all(&record);
    assert!(readme.contains("after"));
    assert!(!readme.contains("old"));
    let _ = fs::remove_file(target);
}

#[test]
fn the_chart_dates_itself_unless_told_not_to() {
    let record = a_record_with_one_day();
    let chart = |dates: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
            .args(["chart", "--activity-record"])
            .arg(&record)
            .args(dates)
            .output()
            .unwrap();
        (
            output.status.success(),
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        )
    };

    let (ok, dated, _) = chart(&[]);
    assert!(ok);
    assert!(
        dated.starts_with("From: 20 August 2026 - To: 20 August 2026"),
        "{dated}"
    );

    let (ok, bare, _) = chart(&["--activity-dates", "off"]);
    assert!(ok);
    assert!(bare.starts_with("LINES BY LANGUAGE"), "{bare}");

    // A misspelt switch is refused rather than read as off, so that a chart
    // asked to date itself either does or says why not.
    let (ok, _, complaint) = chart(&["--activity-dates", "sometimes"]);
    assert!(!ok);
    assert!(complaint.contains("must be on or off"), "{complaint}");

    let _ = fs::remove_dir_all(&record);
}

#[test]
fn a_chart_can_report_a_period_that_has_already_ended() {
    let record = a_record_with_one_day();
    let chart = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
            .args(["chart", "--activity-record"])
            .arg(&record)
            .args(args)
            .output()
            .unwrap();
        (
            output.status.success(),
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        )
    };

    // The record holds one day, in August. Asked about the thirty days to the end
    // of June, the chart has to report an empty period rather than August's work.
    let (ok, june, _) = chart(&["--activity-as-of", "2026-06-30"]);
    assert!(ok);
    assert!(!june.contains("From:"), "{june}");
    assert!(june.contains("nothing recorded"), "{june}");

    let (ok, august, _) = chart(&["--activity-as-of", "2026-08-20"]);
    assert!(ok);
    assert!(
        august.starts_with("From: 20 August 2026 - To: 20 August 2026"),
        "{august}"
    );

    // A day that is not a day is refused, because reading it as no day at all
    // would answer about the present and look like a period with nothing in it.
    let (ok, _, complaint) = chart(&["--activity-as-of", "last June"]);
    assert!(!ok);
    assert!(complaint.contains("must be a day"), "{complaint}");

    let _ = fs::remove_dir_all(&record);
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

    let record = a_record_with_one_day();
    let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "update-readme",
            "--target",
            target.to_str().unwrap(),
            "--activity-record",
            record.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("missing section marker: <!--START_SECTION:activity-->"));
    let _ = fs::remove_dir_all(&record);
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

    // A message that begins with the flag it is about, which is the kind users
    // want, still names only one flag: everything after the first word is prose.
    let mut parsed = source
        .split('"')
        .filter(|token| token.starts_with("--"))
        .filter_map(|token| token.split_whitespace().next())
        .filter(|option| option.len() > 2)
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

#[test]
fn an_activity_card_is_drawn_from_the_record() {
    let record = a_record_with_one_day();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures/github_user_data.json");
    let target = std::env::temp_dir().join(format!(
        "github-personal-stats-activity-{}.svg",
        std::process::id()
    ));

    let status = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "generate",
            "--card",
            "activity",
            "--fixture",
            fixture.to_str().unwrap(),
            "--activity-record",
            record.to_str().unwrap(),
            "--width",
            "760",
            "--height",
            "auto",
            "--output",
            target.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let svg = fs::read_to_string(&target).unwrap();
    // The card took `--activity-record` and drew nothing from it for a whole
    // release: the flag parsed, the record loaded, and the aggregation handed back
    // an empty comparison anyway.
    assert!(!svg.contains("No activity recorded yet"), "{svg}");
    assert!(svg.contains(">Rust<"), "{svg}");
    assert!(svg.contains(">1 hrs 0 mins<"), "{svg}");

    fs::remove_file(&target).ok();
    fs::remove_dir_all(&record).ok();
}

#[test]
fn an_activity_card_asks_github_for_nothing() {
    let record = a_record_with_one_day();
    let target = std::env::temp_dir().join(format!(
        "github-personal-stats-activity-alone-{}.svg",
        std::process::id()
    ));

    let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "generate",
            "--card",
            "activity",
            "--activity-record",
            record.to_str().unwrap(),
            "--width",
            "275",
            "--height",
            "auto",
            "--output",
            target.to_str().unwrap(),
        ])
        // No token, no saved profile, and a proxy that would fail any request
        // made: this card is drawn from the record alone, so a fetch here would
        // be a fetch nobody asked for.
        .env_remove("GITHUB_TOKEN")
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let svg = fs::read_to_string(&target).unwrap();
    assert!(svg.contains(">Rust<"), "{svg}");

    fs::remove_file(&target).ok();
    fs::remove_dir_all(&record).ok();
}

#[test]
fn an_activity_card_says_it_needs_a_record() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures/github_user_data.json");
    let output = Command::new(env!("CARGO_BIN_EXE_github-personal-stats"))
        .args([
            "generate",
            "--card",
            "activity",
            "--fixture",
            fixture.to_str().unwrap(),
            "--output",
            "/dev/null",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--activity-record is needed"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
