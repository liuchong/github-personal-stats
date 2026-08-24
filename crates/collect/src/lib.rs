pub mod clock;
pub mod cursor;
pub mod error;
pub mod language;
pub mod machine;

use std::{collections::BTreeMap, fs, path::PathBuf};

use github_personal_stats_core::{
    ActivitySnapshot, DayBucket, parse_activity_snapshot, write_activity_snapshot,
};

pub use error::CollectError;

pub const DEFAULT_IDLE_TIMEOUT_MINUTES: i64 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub home: PathBuf,
    pub state_dir: PathBuf,
    pub snapshot: PathBuf,
    pub idle_timeout_seconds: i64,
}

pub fn collect(settings: &Settings) -> Result<ActivitySnapshot, CollectError> {
    let machine = machine::identity(&settings.state_dir)?;
    let kept = surviving_days(&settings.snapshot)?;
    let fresh = cursor::read(
        &cursor::database_path(&settings.home),
        settings.idle_timeout_seconds,
    )?;

    let mut days = kept;
    for (date, bucket) in fresh {
        days.insert(date, bucket);
    }

    let mut snapshot = ActivitySnapshot::new(machine, clock::utc_timestamp(clock::now()));
    snapshot.days = days.into_values().collect();
    Ok(snapshot)
}

pub fn save(snapshot: &ActivitySnapshot, path: &PathBuf) -> Result<(), CollectError> {
    let body = write_activity_snapshot(snapshot)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| CollectError::Unreadable {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    fs::write(path, body).map_err(|error| CollectError::Unreadable {
        path: path.clone(),
        message: error.to_string(),
    })
}

fn surviving_days(path: &PathBuf) -> Result<BTreeMap<String, DayBucket>, CollectError> {
    let Ok(existing) = fs::read_to_string(path) else {
        return Ok(BTreeMap::new());
    };
    let snapshot = parse_activity_snapshot(&existing)?;
    Ok(snapshot
        .days
        .into_iter()
        .map(|day| (day.date.clone(), day))
        .collect())
}
