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
pub mod transcripts;

use std::{collections::BTreeMap, path::PathBuf};

use github_personal_stats_core::{ActivitySnapshot, DayBucket, MEASURE_AGENT, MEASURE_EDITOR};

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
    let editor = cursor::read(&cursor::database_path(&settings.home))?;
    let mut days: BTreeMap<String, DayBucket> = editor.days;

    // Terminal agents keep their own records: tokens nothing else can see, and
    // timestamps for work that never passes through an editor.
    let mut moments = editor.moments;
    moments.extend(transcripts::read(&settings.home, &mut days)?);

    // One timeline, not one per source. An afternoon in which an agent ran in the
    // editor and another in a terminal is one afternoon, and the only way to say
    // so is to sort every moment together and let the gaps fall where they fall.
    // Two sources counting their own hours would count that afternoon twice; two
    // sources here instead bound each other's gaps, which is the whole reason the
    // figure gets better rather than merely larger.
    moments.sort_by_key(|moment| moment.second);
    for (date, worked) in sessions::accumulate(&moments, settings.idle_timeout_seconds) {
        *days
            .entry(date.clone())
            .or_insert_with(|| DayBucket::new(&date))
            .measure_mut(MEASURE_AGENT) = worked;
    }

    // Editor time is a different quantity and stays a separate measure. Cursor's
    // store knows what an agent changed; the editor plugins know when someone was
    // present. The two are never added — an agent working while its operator
    // watches is time in both, and summing them would make a day longer than a
    // day.
    for (date, present) in pulse::read(&settings.state_dir, settings.idle_timeout_seconds)? {
        *days
            .entry(date.clone())
            .or_insert_with(|| DayBucket::new(&date))
            .measure_mut(MEASURE_EDITOR) = present;
    }

    let mut snapshot = ActivitySnapshot::new(machine, clock::utc_timestamp(clock::now()));
    snapshot.days = days.into_values().collect();
    Ok(snapshot)
}
