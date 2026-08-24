use std::path::PathBuf;

use github_personal_stats_daemon::service;

fn contents(arguments: &[&str]) -> String {
    service::contents(
        &PathBuf::from("/usr/local/bin/github-personal-stats-daemon"),
        &arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect::<Vec<_>>(),
        &PathBuf::from("/var/log/daemon.log"),
    )
}

#[test]
fn the_service_starts_serve_with_the_options_the_install_was_given() {
    let body = contents(&["--interval", "30", "--sink", "git"]);

    assert!(body.contains("serve"));
    assert!(body.contains("--interval"));
    assert!(body.contains("30"));
    assert!(body.contains("--sink"));
    assert!(body.contains("git"));
}

#[test]
fn the_service_names_the_binary_by_absolute_path() {
    // A launch agent's environment is not a login shell's, so a bare name would
    // not be found.
    assert!(contents(&[]).contains("/usr/local/bin/github-personal-stats-daemon"));
}

#[test]
fn a_path_with_a_space_survives_the_service_description() {
    let body = service::contents(
        &PathBuf::from("/Applications/My Tools/daemon"),
        &[
            "--output".to_owned(),
            "/Users/me/My Files/activity.json".to_owned(),
        ],
        &PathBuf::from("/tmp/daemon.log"),
    );

    if cfg!(target_os = "macos") {
        // Each argument is its own element, so spaces need no quoting.
        assert!(body.contains("<string>/Users/me/My Files/activity.json</string>"));
    } else {
        // One command line, so spaces do need quoting.
        assert!(body.contains("'/Users/me/My Files/activity.json'"));
    }
}

#[test]
fn a_path_with_xml_in_it_cannot_break_the_plist() {
    let body = service::contents(
        &PathBuf::from("/bin/daemon"),
        &["--output".to_owned(), "/tmp/<hack>&.json".to_owned()],
        &PathBuf::from("/tmp/daemon.log"),
    );

    if cfg!(target_os = "macos") {
        assert!(body.contains("&lt;hack&gt;&amp;.json"));
        assert!(!body.contains("<hack>"));
    }
}

#[test]
fn the_service_is_restarted_if_it_stops() {
    let body = contents(&[]);

    if cfg!(target_os = "macos") {
        assert!(body.contains("KeepAlive"));
        assert!(body.contains("RunAtLoad"));
    } else {
        assert!(body.contains("Restart=always"));
        assert!(body.contains("WantedBy=default.target"));
    }
}

#[test]
fn installing_and_removing_name_the_same_service() {
    let service = service::describe().expect("HOME is set while testing");

    assert!(
        service.path.to_string_lossy().contains(service::LABEL),
        "the file should be named after the service: {}",
        service.path.display()
    );
    assert!(service.load.iter().any(|word| !word.is_empty()));
    assert!(service.unload.iter().any(|word| !word.is_empty()));
}
