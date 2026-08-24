use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{CodingActivityEntry, GithubStatsError};

pub const ACTIVITY_SCHEMA: u32 = 1;

const DATE_LENGTH: usize = 10;
const TIMESTAMP_LENGTH: usize = 20;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LineCounts {
    pub agent_added: u64,
    pub agent_deleted: u64,
    pub tab_added: u64,
    pub tab_deleted: u64,
    pub human_added: u64,
    pub human_deleted: u64,
    pub blank_added: u64,
    pub blank_deleted: u64,
    pub unattributed_added: u64,
    pub unattributed_deleted: u64,
}

impl LineCounts {
    pub fn ai_added(&self) -> u64 {
        self.agent_added + self.tab_added
    }

    pub fn ai_deleted(&self) -> u64 {
        self.agent_deleted + self.tab_deleted
    }

    pub fn attributed_added(&self) -> u64 {
        self.ai_added() + self.human_added
    }

    pub fn attributed_deleted(&self) -> u64 {
        self.ai_deleted() + self.human_deleted
    }

    pub fn attributed(&self) -> u64 {
        self.attributed_added() + self.attributed_deleted()
    }

    pub fn added(&self) -> u64 {
        self.attributed_added() + self.blank_added + self.unattributed_added
    }

    pub fn deleted(&self) -> u64 {
        self.attributed_deleted() + self.blank_deleted + self.unattributed_deleted
    }

    pub fn changed(&self) -> u64 {
        self.added() + self.deleted()
    }

    pub fn ai_share_basis_points(&self) -> u32 {
        let attributed = self.attributed();
        if attributed == 0 {
            return 0;
        }
        let assisted = self.ai_added() + self.ai_deleted();
        u32::try_from(assisted.saturating_mul(10_000) / attributed).unwrap_or(10_000)
    }

    pub fn attributed_share_basis_points(&self) -> u32 {
        let changed = self.changed();
        if changed == 0 {
            return 0;
        }
        u32::try_from(self.attributed().saturating_mul(10_000) / changed).unwrap_or(10_000)
    }

    fn absorb(&mut self, other: &Self) {
        self.agent_added += other.agent_added;
        self.agent_deleted += other.agent_deleted;
        self.tab_added += other.tab_added;
        self.tab_deleted += other.tab_deleted;
        self.human_added += other.human_added;
        self.human_deleted += other.human_deleted;
        self.blank_added += other.blank_added;
        self.blank_deleted += other.blank_deleted;
        self.unattributed_added += other.unattributed_added;
        self.unattributed_deleted += other.unattributed_deleted;
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GeneratedLines {
    pub by_model: BTreeMap<String, u64>,
    pub human: u64,
}

impl GeneratedLines {
    pub fn by_agent(&self) -> u64 {
        self.by_model.values().sum()
    }

    pub fn total(&self) -> u64 {
        self.by_agent() + self.human
    }

    pub fn ai_share_basis_points(&self) -> u32 {
        let total = self.total();
        if total == 0 {
            return 0;
        }
        u32::try_from(self.by_agent().saturating_mul(10_000) / total).unwrap_or(10_000)
    }

    fn absorb(&mut self, other: &Self) {
        self.human += other.human;
        for (model, lines) in &other.by_model {
            *self.by_model.entry(model.clone()).or_default() += lines;
        }
    }
}

/// One way of measuring time spent. Two of these live side by side in a day
/// because "the editor was in use" and "code was being changed by an agent" are
/// different quantities, and a day can be long in one and short in the other.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TimeBucket {
    pub seconds: u64,
    pub languages: BTreeMap<String, u64>,
    pub sessions: u32,
}

impl TimeBucket {
    fn absorb(&mut self, other: &Self) {
        self.seconds += other.seconds;
        self.sessions += other.sessions;
        for (language, seconds) in &other.languages {
            *self.languages.entry(language.clone()).or_default() += seconds;
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayBucket {
    pub date: String,
    /// Time the editor was being worked in, reported by editor plugins.
    #[serde(default)]
    pub editor: TimeBucket,
    /// Time in which an agent was changing code, derived from the editor's own
    /// record of what it generated.
    #[serde(default)]
    pub agent: TimeBucket,
    #[serde(default)]
    pub committed: LineCounts,
    #[serde(default)]
    pub generated: GeneratedLines,
    #[serde(default)]
    pub requests: u32,
}

impl DayBucket {
    pub fn new(date: impl Into<String>) -> Self {
        Self {
            date: date.into(),
            ..Self::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.editor.seconds == 0
            && self.agent.seconds == 0
            && self.committed.changed() == 0
            && self.generated.total() == 0
    }

    fn absorb(&mut self, other: &Self) {
        self.editor.absorb(&other.editor);
        self.agent.absorb(&other.agent);
        self.requests += other.requests;
        self.committed.absorb(&other.committed);
        self.generated.absorb(&other.generated);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySnapshot {
    pub schema: u32,
    pub machine: String,
    pub collected_at: String,
    #[serde(default)]
    pub cursors: BTreeMap<String, String>,
    #[serde(default)]
    pub days: Vec<DayBucket>,
}

impl ActivitySnapshot {
    pub fn new(machine: impl Into<String>, collected_at: impl Into<String>) -> Self {
        Self {
            schema: ACTIVITY_SCHEMA,
            machine: machine.into(),
            collected_at: collected_at.into(),
            cursors: BTreeMap::new(),
            days: Vec::new(),
        }
    }

    /// Whether this holds the same record as another, disregarding when it was
    /// collected. Collecting again moves `collected_at` whether or not anything
    /// happened, so a sink that keeps history needs a way to tell a real change
    /// from a clock reading.
    pub fn records_the_same_as(&self, other: &Self) -> bool {
        self.schema == other.schema && self.machine == other.machine && self.days == other.days
    }

    pub fn validate(&self) -> Result<(), GithubStatsError> {
        if self.schema > ACTIVITY_SCHEMA {
            return Err(GithubStatsError::InvalidResponse {
                message: format!(
                    "snapshot schema {} is newer than this build understands ({ACTIVITY_SCHEMA})",
                    self.schema
                ),
            });
        }
        validate_machine(&self.machine)?;
        validate_timestamp(&self.collected_at)?;
        for day in &self.days {
            validate_date(&day.date)?;
        }
        Ok(())
    }
}

pub fn parse_activity_snapshot(input: &str) -> Result<ActivitySnapshot, GithubStatsError> {
    let snapshot = serde_json::from_str::<ActivitySnapshot>(input).map_err(|error| {
        GithubStatsError::InvalidResponse {
            message: format!("could not read activity snapshot: {error}"),
        }
    })?;
    snapshot.validate()?;
    Ok(snapshot)
}

pub fn write_activity_snapshot(snapshot: &ActivitySnapshot) -> Result<String, GithubStatsError> {
    snapshot.validate()?;
    let mut ordered = snapshot.clone();
    ordered
        .days
        .sort_by(|left, right| left.date.cmp(&right.date));
    serde_json::to_string_pretty(&ordered)
        .map(|text| format!("{text}\n"))
        .map_err(|error| GithubStatsError::InvalidResponse {
            message: format!("could not write activity snapshot: {error}"),
        })
}

pub fn merge_snapshots(snapshots: &[ActivitySnapshot]) -> Vec<DayBucket> {
    let mut newest = BTreeMap::<(&str, &str), (&str, &DayBucket)>::new();

    for snapshot in snapshots {
        for day in &snapshot.days {
            let key = (snapshot.machine.as_str(), day.date.as_str());
            match newest.get(&key) {
                Some((seen_at, _)) if *seen_at >= snapshot.collected_at.as_str() => {}
                _ => {
                    newest.insert(key, (snapshot.collected_at.as_str(), day));
                }
            }
        }
    }

    let mut totals = BTreeMap::<&str, DayBucket>::new();
    for ((_, date), (_, day)) in newest {
        totals
            .entry(date)
            .or_insert_with(|| DayBucket::new(date))
            .absorb(day);
    }

    totals.into_values().collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelUsage {
    pub name: String,
    pub lines: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MeasureTotals {
    pub seconds: u64,
    pub sessions: u32,
    pub languages: Vec<CodingActivityEntry>,
    pub daily_seconds: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityTotals {
    pub first_day: Option<String>,
    pub last_day: Option<String>,
    pub active_days: u32,
    pub editor: MeasureTotals,
    pub agent: MeasureTotals,
    pub committed: LineCounts,
    pub generated: GeneratedLines,
    pub models: Vec<ModelUsage>,
    pub requests: u32,
}

pub fn summarise_activity(days: &[DayBucket]) -> ActivityTotals {
    let mut editor_languages = BTreeMap::<&str, u64>::new();
    let mut agent_languages = BTreeMap::<&str, u64>::new();
    let mut models = BTreeMap::<&str, u64>::new();
    let mut totals = ActivityTotals::default();

    for day in days {
        if !day.is_empty() {
            totals.active_days += 1;
        }
        totals.requests += day.requests;
        totals.committed.absorb(&day.committed);
        totals.generated.absorb(&day.generated);
        gather(
            &day.editor,
            &day.date,
            &mut totals.editor,
            &mut editor_languages,
        );
        gather(
            &day.agent,
            &day.date,
            &mut totals.agent,
            &mut agent_languages,
        );
        for (model, lines) in &day.generated.by_model {
            *models.entry(model.as_str()).or_default() += lines;
        }
    }

    totals.first_day = days.first().map(|day| day.date.clone());
    totals.last_day = days.last().map(|day| day.date.clone());
    totals.editor.languages = entries(editor_languages);
    totals.agent.languages = entries(agent_languages);
    totals.models = rank(models)
        .into_iter()
        .map(|(name, lines)| ModelUsage {
            name: name.to_string(),
            lines,
        })
        .collect();
    totals
}

fn gather<'day>(
    bucket: &'day TimeBucket,
    date: &str,
    totals: &mut MeasureTotals,
    languages: &mut BTreeMap<&'day str, u64>,
) {
    totals.seconds += bucket.seconds;
    totals.sessions += bucket.sessions;
    totals.daily_seconds.push((date.to_owned(), bucket.seconds));
    for (language, seconds) in &bucket.languages {
        *languages.entry(language.as_str()).or_default() += seconds;
    }
}

fn entries(counts: BTreeMap<&str, u64>) -> Vec<CodingActivityEntry> {
    rank(counts)
        .into_iter()
        .map(|(language, seconds)| CodingActivityEntry {
            language: language.to_string(),
            seconds,
        })
        .collect()
}

fn rank(counts: BTreeMap<&str, u64>) -> Vec<(&str, u64)> {
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    ranked
}

fn validate_machine(machine: &str) -> Result<(), GithubStatsError> {
    let usable = !machine.is_empty()
        && machine.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        });
    if usable {
        return Ok(());
    }
    Err(GithubStatsError::InvalidResponse {
        message: format!(
            "machine id {machine:?} must be lowercase letters, digits, and dashes so it can name a file"
        ),
    })
}

fn validate_timestamp(value: &str) -> Result<(), GithubStatsError> {
    if value.len() == TIMESTAMP_LENGTH && value.ends_with('Z') {
        return Ok(());
    }
    Err(GithubStatsError::InvalidResponse {
        message: format!("collected_at {value:?} must look like 2026-08-24T19:00:00Z"),
    })
}

fn validate_date(value: &str) -> Result<(), GithubStatsError> {
    let shaped = value.len() == DATE_LENGTH
        && value
            .chars()
            .enumerate()
            .all(|(index, character)| match index {
                4 | 7 => character == '-',
                _ => character.is_ascii_digit(),
            });
    if shaped {
        return Ok(());
    }
    Err(GithubStatsError::InvalidResponse {
        message: format!("day {value:?} must look like 2026-08-24"),
    })
}
