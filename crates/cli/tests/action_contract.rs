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
