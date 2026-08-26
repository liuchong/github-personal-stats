//! The pulse protocol: what an editor plugin tells the daemon, and how it is
//! kept on disk.
//!
//! A pulse says no more than "at this second, on this day, a file of this kind
//! was being worked on". There is no path, no project name, no repository and no
//! file content in it, so neither the journal nor anything derived from it can
//! disclose where you work or what you are building. That boundary is the reason
//! the plugin, rather than the daemon, decides what a file's kind is: it can look
//! at the whole path and send only the extension.
//!
//! The day is also the plugin's to report. The editor knows the machine's local
//! date, and recording the day as it was observed means a journal replayed later,
//! or under a changed timezone, still lands in the day the work happened.

use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use github_personal_stats_core::TimeBucket;
use serde::{Deserialize, Serialize};

use crate::{error::CollectError, sessions};

const JOURNAL_DIR: &str = "pulses";
const DATE_LENGTH: usize = 10;
const MAX_EXTENSION: usize = 24;
const MAX_EDITOR: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pulse {
    /// Seconds since the epoch, used for measuring gaps between pulses.
    pub at: i64,
    /// The local date the pulse belongs to, as the editor saw it.
    pub day: String,
    /// A file extension with no leading dot, lowercased. Empty is allowed and
    /// counts as an unknown kind of file: a window showing something that is not
    /// a file is still a window somebody is at.
    #[serde(default)]
    pub ext: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PulseBatch {
    /// Which editor is reporting, for example `vscode` or `neovim`.
    pub editor: String,
    pub pulses: Vec<Pulse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    at: i64,
    editor: String,
    #[serde(default)]
    ext: String,
}

impl PulseBatch {
    /// Rejects a batch that carries anything the journal is not allowed to hold,
    /// or that could name a file outside the journal directory.
    pub fn validate(&self) -> Result<(), CollectError> {
        if !named_safely(&self.editor, MAX_EDITOR) {
            return Err(rejected(format!(
                "editor {:?} must be lowercase letters, digits and dashes",
                self.editor
            )));
        }
        if self.pulses.is_empty() {
            return Err(rejected("a batch with no pulses in it".to_owned()));
        }
        for pulse in &self.pulses {
            if !dated(&pulse.day) {
                return Err(rejected(format!(
                    "day {:?} must look like 2026-08-24",
                    pulse.day
                )));
            }
            if pulse.at <= 0 {
                return Err(rejected(format!("timestamp {} is not a time", pulse.at)));
            }
            if !pulse.ext.is_empty() && !named_safely(&pulse.ext, MAX_EXTENSION) {
                return Err(rejected(format!(
                    "extension {:?} must be lowercase letters, digits and dashes",
                    pulse.ext
                )));
            }
        }
        Ok(())
    }
}

pub fn journal_directory(state_dir: &Path) -> PathBuf {
    state_dir.join(JOURNAL_DIR)
}

/// What the journal knows about one editor's reporting: when it was last heard
/// from and how many pulses it has sent on a given day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reporter {
    pub editor: String,
    pub pulses: usize,
    pub last_seen: i64,
}

/// Who has reported on a day, most recently heard from first. Answering "is
/// anything being collected" from the journal rather than from a running process
/// means the answer survives a restart, and means a plugin that cannot reach the
/// daemon is visibly absent rather than silently assumed present.
pub fn reporters(state_dir: &Path, day: &str) -> Vec<Reporter> {
    let path = journal_directory(state_dir).join(format!("{day}.jsonl"));
    let Ok(body) = fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut seen = BTreeMap::<String, Reporter>::new();
    for line in body.lines() {
        let Ok(entry) = serde_json::from_str::<Entry>(line) else {
            continue;
        };
        let reporter = seen.entry(entry.editor.clone()).or_insert(Reporter {
            editor: entry.editor,
            pulses: 0,
            last_seen: 0,
        });
        reporter.pulses += 1;
        reporter.last_seen = reporter.last_seen.max(entry.at);
    }

    let mut reporters = seen.into_values().collect::<Vec<_>>();
    reporters.sort_by_key(|reporter| std::cmp::Reverse(reporter.last_seen));
    reporters
}

/// Appends a batch to the journal, one file per day and one line per pulse.
/// Append-only because it is the record of what was observed: aggregation reads
/// it and never rewrites it, so a crash costs at most the pulses in flight.
pub fn append(state_dir: &Path, batch: &PulseBatch) -> Result<usize, CollectError> {
    batch.validate()?;

    let directory = journal_directory(state_dir);
    fs::create_dir_all(&directory).map_err(|error| unreadable(&directory, error))?;

    let mut by_day = BTreeMap::<&str, String>::new();
    for pulse in &batch.pulses {
        let entry = Entry {
            at: pulse.at,
            editor: batch.editor.clone(),
            ext: pulse.ext.clone(),
        };
        let line = serde_json::to_string(&entry)
            .map_err(|error| rejected(format!("a pulse could not be written down: {error}")))?;
        by_day
            .entry(pulse.day.as_str())
            .or_default()
            .push_str(&format!("{line}\n"));
    }

    for (day, lines) in by_day {
        let path = directory.join(format!("{day}.jsonl"));
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| unreadable(&path, error))?;
        file.write_all(lines.as_bytes())
            .map_err(|error| unreadable(&path, error))?;
    }

    Ok(batch.pulses.len())
}

/// Reads the whole journal into editor time, one bucket per day, using the same
/// idle rule as agent time.
pub fn read(
    state_dir: &Path,
    idle_timeout_seconds: i64,
) -> Result<BTreeMap<String, TimeBucket>, CollectError> {
    let directory = journal_directory(state_dir);
    let Ok(entries) = fs::read_dir(&directory) else {
        return Ok(BTreeMap::new());
    };

    let mut events = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| unreadable(&directory, error))?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let Some(day) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if !dated(day) {
            continue;
        }
        let body = fs::read_to_string(&path).map_err(|error| unreadable(&path, error))?;
        for line in body.lines().filter(|line| !line.trim().is_empty()) {
            let entry = serde_json::from_str::<Entry>(line)
                .map_err(|error| rejected(format!("a journal line could not be read: {error}")))?;
            events.push(sessions::Event::new(
                entry.at,
                day,
                crate::language::from_extension(&entry.ext),
            ));
        }
    }

    events.sort_by_key(|event| event.second);
    Ok(sessions::accumulate(&events, idle_timeout_seconds))
}

fn named_safely(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

fn dated(value: &str) -> bool {
    value.len() == DATE_LENGTH
        && value
            .chars()
            .enumerate()
            .all(|(index, character)| match index {
                4 | 7 => character == '-',
                _ => character.is_ascii_digit(),
            })
}

fn rejected(message: String) -> CollectError {
    CollectError::Rejected { message }
}

fn unreadable(path: &Path, error: std::io::Error) -> CollectError {
    CollectError::Unreadable {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}
