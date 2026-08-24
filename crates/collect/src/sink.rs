//! Where a snapshot goes once it has been built.
//!
//! Local collection and publication are separate concerns: the machine that can
//! read the editor's records is not the machine that renders the cards, and the
//! two are connected by a file arriving somewhere the renderer can reach it.
//!
//! Each machine writes its own file, named after its own id. Two machines
//! therefore never touch the same path, which is what makes a shared git
//! repository workable without a merge strategy: there is nothing to merge,
//! because the reader adds the files up itself.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use github_personal_stats_core::{
    ActivitySnapshot, parse_activity_snapshot, write_activity_snapshot,
};

use crate::error::CollectError;

const SNAPSHOT_DIR: &str = "snapshots";

pub trait Sink {
    /// Puts the snapshot where whoever renders the cards will find it.
    fn publish(&self, snapshot: &ActivitySnapshot) -> Result<PathBuf, CollectError>;
}

/// Writes the snapshot to one path and stops there. This is the sink for a
/// machine that renders its own cards, or that hands the file onward by some
/// other means.
#[derive(Debug, Clone)]
pub struct FileSink {
    pub path: PathBuf,
}

impl Sink for FileSink {
    fn publish(&self, snapshot: &ActivitySnapshot) -> Result<PathBuf, CollectError> {
        write_to(&self.path, snapshot)?;
        Ok(self.path.clone())
    }
}

/// Commits the snapshot into a checkout of a data repository and pushes it.
///
/// Shelling out to `git` rather than linking a library is deliberate: it uses the
/// credentials, ssh agent and configuration the user already has working, so a
/// private repository needs no token minted for this and no secret kept anywhere
/// but where git already keeps it.
#[derive(Debug, Clone)]
pub struct GitSink {
    /// A checkout of the data repository. Not the profile repository: activity
    /// history has its own lifetime and its own visibility.
    pub repo: PathBuf,
    pub branch: String,
    /// Whether to push after committing. Off is useful for a machine that is
    /// offline, or for seeing what would be committed first.
    pub push: bool,
}

impl Sink for GitSink {
    fn publish(&self, snapshot: &ActivitySnapshot) -> Result<PathBuf, CollectError> {
        if !self.repo.join(".git").exists() {
            return Err(CollectError::NotFound {
                what: "git checkout to publish into",
                path: self.repo.clone(),
            });
        }

        let relative = PathBuf::from(SNAPSHOT_DIR).join(format!("{}.json", snapshot.machine));
        let path = self.repo.join(&relative);

        // Every collection moves `collected_at`, so the file always differs even
        // in an hour when nothing was worked on. Comparing the record itself is
        // what keeps a daemon on a timer from writing a commit every time it
        // wakes up.
        if let Some(published) = read_published(&path) {
            if published.records_the_same_as(snapshot) {
                return Ok(path);
            }
        }

        write_to(&path, snapshot)?;

        let inside = relative.to_string_lossy().to_string();
        self.git(&["add", &inside])?;

        if self
            .git(&["diff", "--cached", "--quiet", "--", &inside])
            .is_ok()
        {
            return Ok(path);
        }

        let message = format!("Record activity through {}", snapshot.collected_at);
        self.git(&["commit", "--quiet", "--message", &message, "--", &inside])?;

        if self.push {
            self.git(&[
                "push",
                "--quiet",
                "origin",
                &format!("HEAD:{}", self.branch),
            ])?;
        }

        Ok(path)
    }
}

impl GitSink {
    fn git(&self, arguments: &[&str]) -> Result<String, CollectError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.repo)
            .args(arguments)
            .output()
            .map_err(|error| CollectError::Unreadable {
                path: self.repo.clone(),
                message: format!("could not run git: {error}"),
            })?;

        if !output.status.success() {
            return Err(CollectError::Rejected {
                message: format!(
                    "git {} failed: {}",
                    arguments.join(" "),
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Builds the sink a command line asked for. Both the collector and the daemon
/// publish, and they agree about what the options mean because they come through
/// here rather than each deciding for itself.
pub fn choose(
    kind: Option<&str>,
    snapshot: &Path,
    repo: Option<&str>,
    branch: Option<&str>,
    push: bool,
) -> Result<Box<dyn Sink + Send + Sync>, CollectError> {
    match kind {
        None | Some("file") => Ok(Box::new(FileSink {
            path: snapshot.to_path_buf(),
        })),
        Some("git") => {
            let repo = repo.ok_or_else(|| CollectError::Rejected {
                message: "the git sink wants --repo pointing at a checkout of your data repository"
                    .to_owned(),
            })?;
            Ok(Box::new(GitSink {
                repo: PathBuf::from(repo),
                branch: branch.unwrap_or("main").to_owned(),
                push,
            }))
        }
        Some(other) => Err(CollectError::Rejected {
            message: format!("no sink called {other:?}; there is file and git"),
        }),
    }
}

// An HTTP sink belongs here too, for a hosted service collecting from many
// machines. It is left unwritten rather than guessed at: the protocol it would
// speak, and what identifies a user to it, are decisions for whoever runs such a
// service, and the trait above is the whole of what the rest of the code needs to
// know about where a snapshot goes.

/// What is already published, if it is readable. An unreadable or unparseable
/// file is treated as absent: the point of reading it is to avoid a needless
/// commit, and failing to read it should cost a commit rather than the run.
fn read_published(path: &Path) -> Option<ActivitySnapshot> {
    let body = fs::read_to_string(path).ok()?;
    parse_activity_snapshot(&body).ok()
}

fn write_to(path: &Path, snapshot: &ActivitySnapshot) -> Result<(), CollectError> {
    let body = write_activity_snapshot(snapshot)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| CollectError::Unreadable {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    fs::write(path, body).map_err(|error| CollectError::Unreadable {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}
