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

    /// Where that is, in words, for a daemon that has to say what it is doing
    /// before it has done it. Reporting the configured file path regardless of
    /// sink would name a file nothing is written to.
    fn describe(&self) -> String;
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

    fn describe(&self) -> String {
        format!("file {}", self.path.display())
    }
}

/// Commits the snapshot into a git repository and pushes it.
///
/// The repository is storage, not a hosting arrangement: anything the collector
/// can reach over git will do, on the public internet or not, and nothing here
/// knows the name of any particular host. Shelling out to `git` rather than
/// linking a library or calling one host's API is what buys that: it uses the
/// credentials, ssh agent and configuration the user already has working, and it
/// keeps the same code working against a self-hosted remote.
#[derive(Debug, Clone)]
pub struct GitSink {
    /// The working checkout. This belongs in the app's own runtime directory
    /// rather than among the user's projects: it is a storage detail that the app
    /// clones, updates and rebases on its own, not somewhere to work.
    pub repo: PathBuf,
    /// Where to clone from when the checkout is absent, and where to push. Absent
    /// means the checkout must already exist and already know its remote.
    pub origin: Option<String>,
    pub branch: String,
    /// Whether to push after committing. Off is useful for a machine that is
    /// offline, or for seeing what would be committed first.
    pub push: bool,
}

impl Sink for GitSink {
    fn publish(&self, snapshot: &ActivitySnapshot) -> Result<PathBuf, CollectError> {
        self.prepare()?;
        // Catch up with the remote before reading what is published, so the
        // comparison below is against what is actually there, and so this
        // machine's commit lands on top of whatever other machines have pushed.
        self.catch_up()?;

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
            self.push_it()?;
        }

        Ok(path)
    }

    fn describe(&self) -> String {
        let where_to = match &self.origin {
            Some(origin) => format!("{} via {}", origin, self.repo.display()),
            None => self.repo.display().to_string(),
        };
        if self.push {
            format!("git {} on {}", where_to, self.branch)
        } else {
            format!("git {} on {}, without pushing", where_to, self.branch)
        }
    }
}

impl GitSink {
    /// Makes sure there is a checkout to work in, cloning one if the app has not
    /// made it yet. Cloning here rather than asking the user to do it is what lets
    /// the checkout live somewhere private and be treated as replaceable.
    fn prepare(&self) -> Result<(), CollectError> {
        if self.repo.join(".git").is_dir() {
            return Ok(());
        }

        let Some(origin) = &self.origin else {
            return Err(CollectError::NotFound {
                what: "git checkout to publish into, and no remote to clone one from",
                path: self.repo.clone(),
            });
        };

        if let Some(parent) = self.repo.parent() {
            fs::create_dir_all(parent).map_err(|error| CollectError::Unreadable {
                path: parent.to_path_buf(),
                message: error.to_string(),
            })?;
        }

        // Cloning a repository with no commits yet succeeds and leaves no branch,
        // which is fine: the first commit here creates one, and pushing names the
        // branch explicitly rather than relying on whatever local one exists.
        let target = self.repo.display().to_string();
        run(None, &["clone", "--quiet", origin, &target]).map_err(|error| {
            CollectError::Rejected {
                message: format!("could not clone {origin}: {error}"),
            }
        })?;
        Ok(())
    }

    /// Brings the checkout up to date with the remote branch, so a commit made
    /// here is a fast-forward for everyone else. Machines write separate files, so
    /// a rebase here has nothing to conflict over.
    fn catch_up(&self) -> Result<(), CollectError> {
        if self.git(&["remote", "get-url", "origin"]).is_err() {
            return Ok(());
        }
        // A remote that cannot be reached is not a reason to lose a collection:
        // the commit stays local and the next run pushes it.
        if self
            .git(&["fetch", "--quiet", "origin", &self.branch])
            .is_err()
        {
            return Ok(());
        }

        let remote = format!("origin/{}", self.branch);
        if self
            .git(&["rev-parse", "--verify", "--quiet", &remote])
            .is_err()
        {
            return Ok(());
        }

        if self
            .git(&["rev-parse", "--verify", "--quiet", "HEAD"])
            .is_err()
        {
            // Nothing committed here yet, so there is no history to preserve and
            // the remote's is simply adopted.
            self.git(&["reset", "--quiet", "--hard", &remote])?;
            return Ok(());
        }

        self.git(&["rebase", "--quiet", &remote]).inspect_err(|_| {
            // Leave no rebase in progress for the next run to trip over.
            let _ = self.git(&["rebase", "--abort"]);
        })?;
        Ok(())
    }

    /// Pushes, and if the remote moved in the meantime, catches up and tries once
    /// more. A second refusal is worth reporting rather than looping over.
    fn push_it(&self) -> Result<(), CollectError> {
        let refspec = format!("HEAD:{}", self.branch);
        if self.git(&["push", "--quiet", "origin", &refspec]).is_ok() {
            return Ok(());
        }
        self.catch_up()?;
        self.git(&["push", "--quiet", "origin", &refspec])?;
        Ok(())
    }

    fn git(&self, arguments: &[&str]) -> Result<String, CollectError> {
        run(Some(&self.repo), arguments).map_err(|message| CollectError::Rejected {
            message: format!("git {} failed: {message}", arguments.join(" ")),
        })
    }
}

/// Runs git, inside a repository or outside one. Cloning has no repository to be
/// inside yet, which is why the directory is optional.
fn run(repo: Option<&Path>, arguments: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    if let Some(repo) = repo {
        command.arg("-C").arg(repo);
    }
    let output = command
        .args(arguments)
        .output()
        .map_err(|error| format!("could not run git: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Builds the sink a command line or configuration asked for. Both the collector
/// and the daemon publish, and they agree about what the options mean because they
/// come through here rather than each deciding for itself.
pub fn choose(
    kind: Option<&str>,
    snapshot: &Path,
    repo: Option<&str>,
    origin: Option<&str>,
    branch: Option<&str>,
    push: bool,
) -> Result<Box<dyn Sink + Send + Sync>, CollectError> {
    match kind {
        None | Some("file") => Ok(Box::new(FileSink {
            path: snapshot.to_path_buf(),
        })),
        Some("git") => {
            let repo = repo.ok_or_else(|| CollectError::Rejected {
                message: "the git sink wants --repo saying where to keep its checkout".to_owned(),
            })?;
            Ok(Box::new(GitSink {
                repo: PathBuf::from(repo),
                origin: origin.map(str::to_owned),
                branch: branch.unwrap_or("master").to_owned(),
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
