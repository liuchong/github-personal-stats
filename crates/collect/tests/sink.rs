use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use github_personal_stats_collect::sink::{FileSink, GitSink, Sink};
use github_personal_stats_core::{ActivitySnapshot, DayBucket};

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("gps-sink-test-{name}"));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a scratch directory should be creatable");
    directory
}

fn snapshot(collected_at: &str, seconds: u64) -> ActivitySnapshot {
    let mut snapshot = ActivitySnapshot::new("m-1234abcd", collected_at);
    let mut day = DayBucket::new("2026-08-24");
    day.agent.seconds = seconds;
    snapshot.days = vec![day];
    snapshot
}

fn repository(root: &Path) -> PathBuf {
    let repo = root.join("data");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "--quiet"]);
    git(&repo, &["config", "user.email", "test@example.invalid"]);
    git(&repo, &["config", "user.name", "Test"]);
    git(
        &repo,
        &["commit", "--quiet", "--allow-empty", "-m", "start"],
    );
    repo
}

fn git(repo: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(arguments)
        .output()
        .expect("git should be runnable");
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn commits(repo: &Path) -> usize {
    git(repo, &["log", "--oneline"]).lines().count()
}

fn sink(repo: &Path) -> GitSink {
    GitSink {
        repo: repo.to_path_buf(),
        origin: None,
        branch: "master".to_owned(),
        push: false,
    }
}

#[test]
fn a_file_sink_writes_where_it_was_told() {
    let root = scratch("file");
    let path = root.join("nested").join("activity.json");

    let written = FileSink { path: path.clone() }
        .publish(&snapshot("2026-08-24T19:00:00Z", 60))
        .unwrap();

    assert_eq!(written, path);
    assert!(fs::read_to_string(&path).unwrap().contains("2026-08-24"));
}

#[test]
fn a_machine_writes_a_file_named_after_itself() {
    let root = scratch("named");
    let repo = repository(&root);

    let written = sink(&repo)
        .publish(&snapshot("2026-08-24T19:00:00Z", 60))
        .unwrap();

    assert_eq!(written, repo.join("snapshots").join("m-1234abcd.json"));
    assert_eq!(commits(&repo), 2);
}

#[test]
fn collecting_again_without_working_again_writes_no_commit() {
    let root = scratch("quiet");
    let repo = repository(&root);
    let sink = sink(&repo);

    sink.publish(&snapshot("2026-08-24T19:00:00Z", 60)).unwrap();
    let after_first = commits(&repo);

    // Only the clock has moved. This is what a daemon on a timer does all night.
    sink.publish(&snapshot("2026-08-24T19:30:00Z", 60)).unwrap();
    sink.publish(&snapshot("2026-08-24T20:00:00Z", 60)).unwrap();

    assert_eq!(commits(&repo), after_first);
    assert_eq!(git(&repo, &["status", "--short"]), "");
}

#[test]
fn collecting_after_working_more_does_write_a_commit() {
    let root = scratch("busy");
    let repo = repository(&root);
    let sink = sink(&repo);

    sink.publish(&snapshot("2026-08-24T19:00:00Z", 60)).unwrap();
    let after_first = commits(&repo);
    sink.publish(&snapshot("2026-08-24T19:30:00Z", 120))
        .unwrap();

    assert_eq!(commits(&repo), after_first + 1);
}

#[test]
fn two_machines_never_touch_the_same_file() {
    let root = scratch("two");
    let repo = repository(&root);
    let sink = sink(&repo);

    let mut laptop = snapshot("2026-08-24T19:00:00Z", 60);
    laptop.machine = "m-laptop".to_owned();
    let mut desktop = snapshot("2026-08-24T19:00:00Z", 120);
    desktop.machine = "m-desktop".to_owned();

    sink.publish(&laptop).unwrap();
    sink.publish(&desktop).unwrap();

    let mut written = fs::read_dir(repo.join("snapshots"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    written.sort();
    assert_eq!(written, ["m-desktop.json", "m-laptop.json"]);
}

#[test]
fn publishing_into_something_that_is_not_a_checkout_says_so() {
    let root = scratch("bare");

    let refused = sink(&root.join("not-a-repo"))
        .publish(&snapshot("2026-08-24T19:00:00Z", 60))
        .expect_err("a missing checkout should be refused");

    assert!(refused.to_string().contains("git checkout"));
}

#[test]
fn a_snapshot_already_published_by_a_newer_build_is_replaced_rather_than_trusted() {
    let root = scratch("unreadable");
    let repo = repository(&root);
    let path = repo.join("snapshots").join("m-1234abcd.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "{ this is not a snapshot").unwrap();

    sink(&repo)
        .publish(&snapshot("2026-08-24T19:00:00Z", 60))
        .expect("an unreadable file should not stop the run");

    assert!(fs::read_to_string(&path).unwrap().contains("2026-08-24"));
}

fn bare(root: &Path) -> String {
    let remote = root.join("remote.git");
    let output = std::process::Command::new("git")
        .args(["init", "--quiet", "--bare", "--initial-branch=master"])
        .arg(&remote)
        .output()
        .expect("git should be runnable");
    assert!(output.status.success());
    remote.display().to_string()
}

fn cloning(repo: &Path, origin: &str) -> GitSink {
    GitSink {
        repo: repo.to_path_buf(),
        origin: Some(origin.to_owned()),
        branch: "master".to_owned(),
        push: true,
    }
}

#[test]
fn a_checkout_that_is_not_there_yet_is_cloned() {
    let root = scratch("clone");
    let origin = bare(&root);
    let repo = root.join("runtime").join("storage");

    let written = cloning(&repo, &origin)
        .publish(&snapshot("2026-08-24T19:00:00Z", 60))
        .expect("an absent checkout should be created rather than refused");

    assert!(written.starts_with(&repo));
    assert!(repo.join(".git").is_dir());
}

#[test]
fn the_first_snapshot_reaches_an_empty_remote() {
    let root = scratch("empty-remote");
    let origin = bare(&root);
    let repo = root.join("storage");

    cloning(&repo, &origin)
        .publish(&snapshot("2026-08-24T19:00:00Z", 60))
        .unwrap();

    let landed = git(&PathBuf::from(&origin), &["log", "--oneline", "master"]);
    assert_eq!(landed.lines().count(), 1);
}

#[test]
fn a_second_machine_lands_on_top_of_the_first_without_conflict() {
    let root = scratch("two-machines");
    let origin = bare(&root);

    let mut laptop = snapshot("2026-08-24T19:00:00Z", 60);
    laptop.machine = "m-laptop".to_owned();
    let mut desktop = snapshot("2026-08-24T19:05:00Z", 120);
    desktop.machine = "m-desktop".to_owned();

    // Two checkouts that know nothing of each other, as two machines would be.
    cloning(&root.join("one"), &origin)
        .publish(&laptop)
        .unwrap();
    cloning(&root.join("two"), &origin)
        .publish(&desktop)
        .expect("a remote that moved should be caught up with, not fought over");

    let history = git(&PathBuf::from(&origin), &["log", "--oneline", "master"]);
    assert_eq!(history.lines().count(), 2);

    // Both files survive: neither machine overwrote the other's.
    let listing = git(
        &PathBuf::from(&origin),
        &["ls-tree", "--name-only", "master", "snapshots/"],
    );
    assert!(listing.contains("m-laptop.json"), "{listing}");
    assert!(listing.contains("m-desktop.json"), "{listing}");
}

#[test]
fn a_machine_that_cannot_reach_the_remote_still_records_locally() {
    let root = scratch("offline");
    let origin = bare(&root);
    let repo = root.join("storage");
    cloning(&repo, &origin)
        .publish(&snapshot("2026-08-24T19:00:00Z", 60))
        .unwrap();

    // The remote goes away, as it does on a train.
    fs::remove_dir_all(&origin).unwrap();

    let sink = GitSink {
        repo: repo.clone(),
        origin: Some(origin),
        branch: "master".to_owned(),
        push: false,
    };
    sink.publish(&snapshot("2026-08-24T19:30:00Z", 120))
        .expect("an unreachable remote should not lose the collection");

    // Both collections are committed and waiting; the next run with a reachable
    // remote pushes them together.
    assert_eq!(commits(&repo), 2);
}
