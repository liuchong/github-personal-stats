//! The published record on disk, and how a collection is folded into it.
//!
//! A collection is a reading of sources that forget. Cursor's own store keeps
//! roughly a month, so the collection made today describes today well, describes
//! last week well, and describes April not at all. Writing a collection out as
//! the record would therefore lose a day's work a month after it was done.
//!
//! So a collection is never written as the record; it is merged into it. Each day
//! is its own file, and a day file is replaced only by a fuller reading of that
//! same day. What survives is the union of every collection ever made, which is a
//! history longer than any source it was read from.
//!
//! Writing only the day files that changed is what this buys on top: a run that
//! finds an hour of new work touches one file, so a reader can fetch a window
//! instead of everything, and a git history of these commits says which day each
//! one recorded instead of showing the whole record rewritten.

use std::{
    fs,
    path::{Path, PathBuf},
};

use github_personal_stats_core::{
    ActivitySnapshot, DayBucket,
    store::{
        DayRecord, MANIFEST, MachineManifest, day_file, keep_fuller_days, machine_directory,
        manifest_file, parse_day, parse_manifest, write_day, write_manifest,
    },
};

use crate::error::CollectError;

/// What a publication left on disk.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Written {
    /// The directory holding the machine's days and manifest.
    pub directory: PathBuf,
    /// Files whose contents changed, relative to the root. A publication that
    /// found no new work leaves this empty, which is what lets a sink on a timer
    /// avoid committing every time it wakes up.
    pub changed: Vec<PathBuf>,
    /// Files no longer part of the record, relative to the root. Only ever the
    /// superseded single-file record.
    pub removed: Vec<PathBuf>,
}

impl Written {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.removed.is_empty()
    }
}

/// Every day a machine has already published.
///
/// An unreadable day file stops the read rather than being skipped. Skipping it
/// would let the next collection write a thinner reading over a day whose only
/// copy was the file that failed to parse, which is the one failure this module
/// exists to prevent. A day file from a newer build fails the same way, and for
/// the same reason.
pub fn read_days(root: &Path, machine: &str) -> Result<Vec<DayBucket>, CollectError> {
    let directory = machine_directory(root, machine);
    let listing = match fs::read_dir(&directory) {
        Ok(listing) => listing,
        // Nothing published yet is not a problem; it is the first run.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(CollectError::Unreadable {
                path: directory,
                message: error.to_string(),
            });
        }
    };

    let mut days = Vec::new();
    for entry in listing {
        let path = entry
            .map_err(|error| CollectError::Unreadable {
                path: directory.clone(),
                message: error.to_string(),
            })?
            .path();

        if path.file_name().and_then(|name| name.to_str()) == Some(MANIFEST) {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let body = fs::read_to_string(&path).map_err(|error| CollectError::Unreadable {
            path: path.clone(),
            message: error.to_string(),
        })?;
        let record = parse_day(&body).map_err(|error| CollectError::Unreadable {
            path: path.clone(),
            message: error.to_string(),
        })?;
        days.push(record.day);
    }
    days.sort_by(|left, right| left.date.cmp(&right.date));
    Ok(days)
}

/// What to do with the time already on record for the days a reading covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recount {
    /// Keep whichever reading saw more, which is right for as long as the way
    /// time is counted stays the same.
    KeepFuller,
    /// Replace it with this reading's, for the days this reading covers.
    ///
    /// For when the counting itself changes rather than the record growing. A
    /// larger figure from the old rule is not a fuller reading of the day, it is
    /// a figure from a rule no longer in force, and keeping the larger of the two
    /// would mean a measure could never be corrected downwards.
    ///
    /// Only time is replaced, and only for days this reading covers, so this
    /// cannot erase lines or commits the sources have since forgotten.
    Replace,
}

/// Folds a collection into the published record, writing only what changed.
///
/// The root holds one directory per machine. Two machines therefore write
/// disjoint sets of paths, which is what makes a shared git repository work
/// without a merge strategy.
pub fn publish(
    root: &Path,
    snapshot: &ActivitySnapshot,
    recount: Recount,
) -> Result<Written, CollectError> {
    let machine = snapshot.machine.as_str();
    let mut written = Written {
        directory: machine_directory(root, machine),
        ..Written::default()
    };

    let mut published = read_days(root, machine)?;
    if let Some(superseded) = supersede(root, machine, &mut published)? {
        written.removed.push(superseded);
    }

    let mut held = keep_fuller_days(&published, &snapshot.days);
    if recount == Recount::Replace {
        recount_time(&mut held, &snapshot.days);
    }

    for day in &held {
        let record = DayRecord::new(machine, day.clone());
        let body = write_day(&record)?;
        let path = day_file(root, machine, &day.date);
        if put(&path, &body)? {
            written.changed.push(relative(root, &path));
        }
    }

    let mut manifest = MachineManifest::new(machine, snapshot.collected_at.clone());
    manifest.cursors = snapshot.cursors.clone();
    let manifest = manifest.describing(&held);

    // The manifest carries `collected_at`, which moves on every run whether or
    // not any work happened. Rewriting it regardless would make a commit out of
    // reading the clock, so it is only written when it describes something new.
    let path = manifest_file(root, machine);
    let stale = match read_manifest(&path) {
        Some(existing) => !manifest.records_the_same_as(&existing),
        None => true,
    };
    if stale && put(&path, &write_manifest(&manifest)?)? {
        written.changed.push(relative(root, &path));
    }

    Ok(written)
}

/// Puts this reading's time on the days it covers, in place of what was there.
///
/// A day the reading did not reach keeps the time it had: the sources forget, and
/// a day they have forgotten is not a day that held no work.
fn recount_time(held: &mut [DayBucket], fresh: &[DayBucket]) {
    for day in held.iter_mut() {
        if let Some(reading) = fresh.iter().find(|other| other.date == day.date) {
            day.time = reading.time.clone();
        }
    }
}

/// Takes in the record written before days were split into files, so that the
/// history it holds is not stranded. It is the only copy of days the sources have
/// since forgotten, so it is read before being removed, and both happen in the
/// same publication so no run sees neither.
fn supersede(
    root: &Path,
    machine: &str,
    published: &mut Vec<DayBucket>,
) -> Result<Option<PathBuf>, CollectError> {
    let path = root.join(format!("{machine}.json"));
    let Ok(body) = fs::read_to_string(&path) else {
        return Ok(None);
    };

    let older = github_personal_stats_core::parse_activity_snapshot(&body).map_err(|error| {
        CollectError::Unreadable {
            path: path.clone(),
            message: error.to_string(),
        }
    })?;
    *published = keep_fuller_days(published, &older.days);

    fs::remove_file(&path).map_err(|error| CollectError::Unreadable {
        path: path.clone(),
        message: error.to_string(),
    })?;
    Ok(Some(relative(root, &path)))
}

fn read_manifest(path: &Path) -> Option<MachineManifest> {
    parse_manifest(&fs::read_to_string(path).ok()?).ok()
}

/// Writes a file unless it already holds exactly this, and says whether it wrote.
/// Comparing before writing is what keeps the caller's list of changed files
/// honest, and keeps a modification time from moving for no reason.
fn put(path: &Path, body: &str) -> Result<bool, CollectError> {
    if fs::read_to_string(path).is_ok_and(|held| held == body) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| CollectError::Unreadable {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    fs::write(path, body)
        .map_err(|error| CollectError::Unreadable {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
        .map(|()| true)
}

fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
