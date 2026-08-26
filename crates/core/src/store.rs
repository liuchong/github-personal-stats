//! How a machine's activity record is laid out on disk.
//!
//! A day's work is one file. The alternative, one growing file per machine, was
//! what this replaced: every collection rewrote the whole thing, so the commit
//! that recorded four hundred bytes of new work carried the entire history with
//! it, and pruning old days meant rewriting rather than deleting.
//!
//! Splitting by day costs a reader that wants a recent window nothing — it opens
//! the hundred files it needs — but it would cost a reader that wants a lifetime
//! total everything, which is the figure this project exists to show. So the
//! manifest carries a rollup alongside the index. The rollup is a cache and is
//! written from the day files, never the other way round; `roll_up` is the only
//! thing that produces one, and a reader that distrusts it can call the same
//! function on the days themselves.
//!
//! The layout also decides how long the record lasts, which is the reason it
//! matters more than tidiness. The editor's own store keeps roughly a month, and
//! every collection rebuilds from it, so a collection made today knows nothing
//! about the day three months ago. A published record that mirrored the latest
//! collection would therefore lose a day's work a month after it was done. A day
//! that is its own file, written once and afterwards only ever replaced by a
//! fuller reading of that same day, cannot lose it: the accumulation is in the
//! layout rather than in code that has to remember to merge.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    GithubStatsError,
    activity::{
        ACTIVITY_SCHEMA, DayBucket, LegacyDay, LineCounts, LineFact, TimeBucket, TokenUsage,
    },
};

/// Names the per-machine file holding identity, the day index, and the rollup.
pub const MANIFEST: &str = "manifest.json";

/// The directory each machine publishes into, named by its own id so that two
/// machines writing at once never touch the same path.
pub fn machine_directory(root: &Path, machine: &str) -> PathBuf {
    root.join(machine)
}

/// The file holding one day of one machine's work.
pub fn day_file(root: &Path, machine: &str, date: &str) -> PathBuf {
    machine_directory(root, machine).join(format!("{date}.json"))
}

/// The file holding one machine's identity, day index, and rollup.
pub fn manifest_file(root: &Path, machine: &str) -> PathBuf {
    machine_directory(root, machine).join(MANIFEST)
}

/// A day as it is stored: the bucket, plus enough to make the file mean
/// something on its own. The machine and schema are repeated in every day file
/// rather than inferred from the directory name, so that a file which has been
/// copied, fetched over HTTP, or moved by hand can still be read and checked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayRecord {
    pub schema: u32,
    pub machine: String,
    #[serde(flatten)]
    pub day: DayBucket,
}

impl DayRecord {
    pub fn new(machine: impl Into<String>, day: DayBucket) -> Self {
        Self {
            schema: ACTIVITY_SCHEMA,
            machine: machine.into(),
            day,
        }
    }
}

/// Everything additive about a span of days, summed once so that a reader after
/// a lifetime total does not have to open every day to get it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Rollup {
    pub first_day: String,
    pub last_day: String,
    /// Days that hold any work at all, which is not the span between the first
    /// and last: a month with two days of work counts two.
    pub active_days: u32,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub time: BTreeMap<String, TimeBucket>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<LineFact>,
    pub commits: LineCounts,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub tokens: BTreeMap<String, TokenUsage>,
    pub requests: u32,
}

impl Rollup {
    /// The named measure, or an empty one, matching how a day is read.
    pub fn measure(&self, name: &str) -> TimeBucket {
        self.time.get(name).cloned().unwrap_or_default()
    }
}

/// Sums days into a rollup. Returns nothing for an empty record rather than a
/// zeroed rollup, because a first and last day would have to be invented.
///
/// A rollup is a cache and nothing more. Every figure in it can be recomputed
/// from the day files, which is what makes it safe to throw away and rebuild when
/// the shape of a day changes.
pub fn roll_up(days: &[DayBucket]) -> Option<Rollup> {
    let mut ordered = days.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.date.cmp(&right.date));
    let first = ordered.first()?;
    let last = ordered.last()?;

    let mut active_days = 0;
    let mut summed = DayBucket::new("");
    for day in &ordered {
        if !day.is_empty() {
            active_days += 1;
        }
        summed.absorb(day);
    }

    Some(Rollup {
        first_day: first.date.clone(),
        last_day: last.date.clone(),
        active_days,
        time: summed.time,
        lines: summed.lines,
        commits: summed.commits,
        tokens: summed.tokens,
        requests: summed.requests,
    })
}

/// A machine's identity, what days it has published, and their sum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineManifest {
    pub schema: u32,
    pub machine: String,
    pub collected_at: String,
    /// Where incremental reads of each source got to, so a later collection can
    /// resume rather than rescan.
    #[serde(default)]
    pub cursors: BTreeMap<String, String>,
    /// The days this machine has published. A reader that can list the directory
    /// does not need this, but one fetching over HTTP cannot list anything.
    #[serde(default)]
    pub days: Vec<String>,
    #[serde(default)]
    pub rollup: Option<Rollup>,
}

impl MachineManifest {
    pub fn new(machine: impl Into<String>, collected_at: impl Into<String>) -> Self {
        Self {
            schema: ACTIVITY_SCHEMA,
            machine: machine.into(),
            collected_at: collected_at.into(),
            cursors: BTreeMap::new(),
            days: Vec::new(),
            rollup: None,
        }
    }

    /// Describes a set of days: indexes them and sums them.
    pub fn describing(mut self, days: &[DayBucket]) -> Self {
        let mut dates = days.iter().map(|day| day.date.clone()).collect::<Vec<_>>();
        dates.sort();
        dates.dedup();
        self.days = dates;
        self.rollup = roll_up(days);
        self
    }

    /// Whether this describes the same record as another, disregarding when it
    /// was collected. Collecting again moves `collected_at` whether or not any
    /// work happened, so a sink that keeps history needs to tell a real change
    /// from a clock reading.
    pub fn records_the_same_as(&self, other: &Self) -> bool {
        self.schema == other.schema
            && self.machine == other.machine
            && self.days == other.days
            && self.rollup == other.rollup
    }

    pub fn validate(&self) -> Result<(), GithubStatsError> {
        check_schema(self.schema)?;
        crate::activity::validate_machine(&self.machine)?;
        crate::activity::validate_timestamp(&self.collected_at)?;
        for date in &self.days {
            crate::activity::validate_date(date)?;
        }
        Ok(())
    }
}

pub fn parse_manifest(input: &str) -> Result<MachineManifest, GithubStatsError> {
    let manifest = serde_json::from_str::<MachineManifest>(input)
        .map_err(|error| unreadable("manifest", error))?;
    manifest.validate()?;
    Ok(manifest)
}

pub fn write_manifest(manifest: &MachineManifest) -> Result<String, GithubStatsError> {
    manifest.validate()?;
    render(manifest)
}

/// Reads a day file, bringing an older one forward.
///
/// The record is years long and outlives the shape a day was first written in, so
/// reading is where the schema history lives: a file says which schema wrote it,
/// and each older schema has one conversion into the current shape. Nothing on
/// disk needs rewriting for a build to understand it.
pub fn parse_day(input: &str) -> Result<DayRecord, GithubStatsError> {
    let stamped =
        serde_json::from_str::<StampedDay>(input).map_err(|error| unreadable("day", error))?;
    check_schema(stamped.schema)?;

    let day = if stamped.schema < ACTIVITY_SCHEMA {
        serde_json::from_str::<LegacyDay>(input)
            .map_err(|error| unreadable("day", error))?
            .bring_forward()
    } else {
        serde_json::from_str::<DayBucket>(input).map_err(|error| unreadable("day", error))?
    };

    crate::activity::validate_machine(&stamped.machine)?;
    crate::activity::validate_date(&day.date)?;
    Ok(DayRecord {
        schema: ACTIVITY_SCHEMA,
        machine: stamped.machine,
        day,
    })
}

/// Just enough of a day file to learn which schema wrote it.
#[derive(Deserialize)]
struct StampedDay {
    schema: u32,
    machine: String,
}

pub fn write_day(record: &DayRecord) -> Result<String, GithubStatsError> {
    check_schema(record.schema)?;
    crate::activity::validate_machine(&record.machine)?;
    crate::activity::validate_date(&record.day.date)?;
    render(record)
}

/// Reads a whole record: every machine's days, summed per date.
///
/// Days from different machines on the same date are added, because they are
/// different work in different places. That is the opposite of how two readings
/// of one machine's day combine, where the fuller reading wins; the difference is
/// what the directory layout encodes, one directory per machine.
///
/// Machines are found by listing the directory rather than by consulting an
/// index, so a record assembled by hand or by copying a directory in works
/// without anything having to be registered.
pub fn read_record(root: &Path) -> Result<Vec<DayBucket>, GithubStatsError> {
    let mut totals = BTreeMap::<String, DayBucket>::new();

    let machines = fs::read_dir(root).map_err(|error| GithubStatsError::InvalidResponse {
        message: format!(
            "could not read the activity record at {}: {error}",
            root.display()
        ),
    })?;

    for machine in machines {
        let machine = machine.map_err(|error| GithubStatsError::InvalidResponse {
            message: format!("could not read the activity record: {error}"),
        })?;
        if !machine.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        for day in day_files(&machine.path())? {
            let content =
                fs::read_to_string(&day).map_err(|error| GithubStatsError::InvalidResponse {
                    message: format!("could not read {}: {error}", day.display()),
                })?;
            let record = parse_day(&content)?;
            totals
                .entry(record.day.date.clone())
                .or_insert_with(|| DayBucket::new(&record.day.date))
                .absorb(&record.day);
        }
    }

    Ok(totals.into_values().collect())
}

/// The day files of one machine, in date order. The manifest is a cache and is
/// skipped; anything else that is not a dated file is ignored rather than
/// treated as a broken day.
fn day_files(directory: &Path) -> Result<Vec<PathBuf>, GithubStatsError> {
    let entries = fs::read_dir(directory).map_err(|error| GithubStatsError::InvalidResponse {
        message: format!("could not read {}: {error}", directory.display()),
    })?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| GithubStatsError::InvalidResponse {
            message: format!("could not read {}: {error}", directory.display()),
        })?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(stem) = name.strip_suffix(".json") else {
            continue;
        };
        if crate::activity::validate_date(stem).is_ok() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Combines readings of the same days into one set, keeping the fuller reading of
/// each day. Days that appear in only one of the two come through untouched.
///
/// This is how a collection is folded into what is already published: the days
/// the collector can still see are refreshed, and the days that have aged out of
/// the source it reads keep the figures they were published with.
pub fn keep_fuller_days(published: &[DayBucket], fresh: &[DayBucket]) -> Vec<DayBucket> {
    let mut held = BTreeMap::new();
    for day in published.iter().chain(fresh) {
        held.entry(day.date.clone())
            .and_modify(|kept: &mut DayBucket| kept.keep_fuller(day))
            .or_insert_with(|| day.clone());
    }
    held.into_values().collect()
}

fn render<T: Serialize>(value: &T) -> Result<String, GithubStatsError> {
    serde_json::to_string_pretty(value)
        .map(|text| format!("{text}\n"))
        .map_err(|error| GithubStatsError::InvalidResponse {
            message: format!("could not write activity record: {error}"),
        })
}

fn unreadable(what: &str, error: serde_json::Error) -> GithubStatsError {
    GithubStatsError::InvalidResponse {
        message: format!("could not read activity {what}: {error}"),
    }
}

fn check_schema(schema: u32) -> Result<(), GithubStatsError> {
    if schema > ACTIVITY_SCHEMA {
        return Err(GithubStatsError::InvalidResponse {
            message: format!(
                "activity schema {schema} is newer than this build understands ({ACTIVITY_SCHEMA})"
            ),
        });
    }
    Ok(())
}
