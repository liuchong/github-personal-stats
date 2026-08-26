//! Where a snapshot goes once it has been built.
//!
//! Local collection and publication are separate concerns: the machine that can
//! read the editor's records is not the machine that renders the cards, and the
//! two are connected by a file arriving somewhere the renderer can reach it.
//!
//! Each machine writes its own directory, named after its own id. Two machines
//! therefore never touch the same path, which is what makes a shared git
//! repository workable without a merge strategy: there is nothing to merge,
//! because the reader adds the files up itself.
//!
//! What goes in that directory, and how a collection is folded into what is
//! already there rather than replacing it, is `records`. A sink decides where the
//! root is and what to do afterwards; it does not decide the shape.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use github_personal_stats_core::ActivitySnapshot;

use crate::{error::CollectError, records};

const SNAPSHOT_DIR: &str = "snapshots";

pub trait Sink {
    /// Puts the snapshot where whoever renders the cards will find it.
    fn publish(&self, snapshot: &ActivitySnapshot) -> Result<PathBuf, CollectError>;

    /// Where that is, in words, for a daemon that has to say what it is doing
    /// before it has done it. Reporting the configured file path regardless of
    /// sink would name a file nothing is written to.
    fn describe(&self) -> String;

    /// The root the record can be read back from, for a caller that wants to show
    /// what has been collected without publishing to find out. The record holds
    /// years; a fresh reading holds a month, so the two are not interchangeable.
    fn root(&self) -> PathBuf;
}

/// Writes into one directory and stops there. This is the sink for a machine that
/// renders its own cards, or that hands the record onward by some other means.
#[derive(Debug, Clone)]
pub struct FileSink {
    /// The root the machine's directory of days goes under.
    pub path: PathBuf,
}

impl Sink for FileSink {
    fn publish(&self, snapshot: &ActivitySnapshot) -> Result<PathBuf, CollectError> {
        let written = records::publish(&self.path, snapshot)?;
        Ok(written.directory)
    }

    fn describe(&self) -> String {
        format!("files under {}", self.path.display())
    }

    fn root(&self) -> PathBuf {
        self.path.clone()
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

        let root = self.repo.join(SNAPSHOT_DIR);
        let written = records::publish(&root, snapshot)?;

        // A publication that changed nothing is what a daemon on a timer produces
        // most of the time, and it must not become a commit.
        if written.is_empty() {
            return Ok(written.directory);
        }

        let inside = |path: &Path| {
            Path::new(SNAPSHOT_DIR)
                .join(path)
                .to_string_lossy()
                .into_owned()
        };
        let touched = written
            .changed
            .iter()
            .chain(&written.removed)
            .map(|path| inside(path))
            .collect::<Vec<_>>();

        let mut add = vec!["add", "--all", "--"];
        add.extend(touched.iter().map(String::as_str));
        self.git(&add)?;

        let mut staged = vec!["diff", "--cached", "--quiet", "--"];
        staged.extend(touched.iter().map(String::as_str));
        if self.git(&staged).is_ok() {
            return Ok(written.directory);
        }

        let message = describe_change(&written, snapshot);
        let identity = self.identity();
        let mut commit = identity.iter().map(String::as_str).collect::<Vec<_>>();
        commit.extend(["commit", "--quiet", "--message", &message, "--"]);
        commit.extend(touched.iter().map(String::as_str));
        self.git(&commit)?;

        if self.push {
            self.push_it()?;
        }

        Ok(written.directory)
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

    fn root(&self) -> PathBuf {
        self.repo.join(SNAPSHOT_DIR)
    }
}

impl GitSink {
    /// Who to record as the author, when git would otherwise refuse to commit.
    ///
    /// git will not commit without an identity, and a collector running in the
    /// background has no business requiring one to have been configured globally
    /// first. A machine that has one keeps it; a machine that has none gets the
    /// tool's name, which is honest about what made the commit. Passed per command
    /// rather than written into the checkout's configuration, so nothing outside
    /// this commit is affected.
    fn identity(&self) -> Vec<String> {
        if self.git(&["config", "user.email"]).is_ok() {
            return Vec::new();
        }
        vec![
            "-c".to_owned(),
            "user.name=github-personal-stats".to_owned(),
            "-c".to_owned(),
            "user.email=github-personal-stats@localhost".to_owned(),
        ]
    }

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

/// Names the days a commit recorded, because that is what the history of this
/// repository is for. A record rewritten whole every half hour produces commits
/// that all look alike and none of which can be read; one day per file means the
/// subject line can say which day changed.
fn describe_change(written: &records::Written, snapshot: &ActivitySnapshot) -> String {
    let days = written
        .changed
        .iter()
        .filter_map(|path| path.file_stem()?.to_str())
        .filter(|name| *name != "manifest")
        .collect::<Vec<_>>();

    match days.as_slice() {
        [] => format!("Reindex activity for {}", snapshot.machine),
        [day] => format!("Record activity for {day}"),
        [first, .., last] => {
            format!("Record activity for {} days, {first} to {last}", days.len())
        }
    }
}
