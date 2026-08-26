pub mod clock;
pub mod cursor;
pub mod error;
pub mod language;
pub mod machine;
pub mod preferences;
pub mod presence;
pub mod pulse;
pub mod random;
pub mod records;
pub mod sessions;
pub mod sink;

use std::{collections::BTreeMap, path::PathBuf};

use github_personal_stats_core::{ActivitySnapshot, DayBucket, MEASURE_EDITOR};

pub use error::CollectError;

pub const DEFAULT_IDLE_TIMEOUT_MINUTES: i64 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub home: PathBuf,
    pub state_dir: PathBuf,
    /// The root the record is published under. One directory per machine goes
    /// here, and one file per day inside that.
    pub snapshot: PathBuf,
    pub idle_timeout_seconds: i64,
}

/// Reads what the local sources say right now.
///
/// This is a reading, not the record. It reaches back only as far as the sources
/// do — Cursor keeps roughly a month — and it deliberately does not consult what
/// has already been published. Growing a history out of successive readings is
/// `records::publish`, which folds a reading into the days already on disk and
/// keeps whichever reading of each day saw more.
///
/// Keeping the two apart is what makes a reading safe to take at any time: it
/// cannot shrink a record it never opened.
pub fn collect(settings: &Settings) -> Result<ActivitySnapshot, CollectError> {
    let machine = machine::identity(&settings.state_dir)?;
    let fresh = cursor::read(
        &cursor::database_path(&settings.home),
        settings.idle_timeout_seconds,
    )?;

    let worked = pulse::read(&settings.state_dir, settings.idle_timeout_seconds)?;

    // The two sources measure different things and neither can fill in for the
    // other: Cursor's store knows what an agent changed, and the editor plugins
    // know when someone was present. So each owns a measure of its own, and the
    // two are never added — an agent working while its operator watches is time
    // in both, and summing them would make a day longer than a day.
    let mut days: BTreeMap<String, DayBucket> = fresh.into_iter().collect();
    for (date, editor) in worked {
        *days
            .entry(date.clone())
            .or_insert_with(|| DayBucket::new(&date))
            .measure_mut(MEASURE_EDITOR) = editor;
    }

    let mut snapshot = ActivitySnapshot::new(machine, clock::utc_timestamp(clock::now()));
    snapshot.days = days.into_values().collect();
    Ok(snapshot)
}
