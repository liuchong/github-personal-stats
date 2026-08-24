use std::{fs, path::Path};

#[test]
fn action_uses_binary_download_without_rust_build_steps() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let action = fs::read_to_string(root.join("action.yml")).unwrap();
    let installer = fs::read_to_string(root.join("scripts/install-action-binary.sh")).unwrap();

    assert!(action.contains("scripts/install-action-binary.sh"));
    assert!(installer.contains("releases/latest/download"));
    assert!(!action.contains("cargo build"));
    assert!(!action.contains("cargo install"));
    assert!(!action.contains("rustup"));
}

#[test]
fn the_action_offers_a_mode_for_every_command_it_can_run() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let action = fs::read_to_string(root.join("action.yml")).unwrap();

    for (mode, command) in [
        ("fetch", "github-personal-stats fetch"),
        ("update-readme", "github-personal-stats update-readme"),
    ] {
        assert!(
            action.contains(&format!(r#""${{INPUT_MODE}}" = "{mode}""#)),
            "no branch for mode {mode}"
        );
        assert!(action.contains(command), "mode {mode} runs nothing");
    }
    assert!(action.contains("github-personal-stats generate"));

    // An input the run script never reads looks like a feature and is not one.
    let declared = action
        .split_once("runs:")
        .unwrap()
        .0
        .lines()
        .filter_map(|line| line.strip_suffix(':')?.strip_prefix("  "))
        .filter(|name| !name.contains(' '))
        .collect::<Vec<_>>();
    assert!(declared.len() > 5, "found only {declared:?}");
    for input in declared {
        let referenced = format!("inputs.{input}");
        assert!(
            action.matches(&referenced).count() > 0,
            "input {input} is never used"
        );
    }
}
