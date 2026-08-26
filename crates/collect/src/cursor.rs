use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};

use github_personal_stats_core::{Author, DayBucket, MEASURE_AGENT};
use rusqlite::{Connection, OpenFlags};

use crate::{
    error::CollectError,
    language,
    sessions::{self, Event},
};

const SCORED_COMMITS: &str = "scored_commits";
const CODE_HASHES: &str = "ai_code_hashes";

/// How long to wait for the editor to finish writing before giving up on a read.
const LOCK_WAIT: Duration = Duration::from_secs(30);

const COMMITTED_QUERY: &str = "SELECT commitDate, \
     tabLinesAdded, tabLinesDeleted, \
     composerLinesAdded, composerLinesDeleted, \
     humanLinesAdded, humanLinesDeleted, \
     blankLinesAdded, blankLinesDeleted, \
     linesAdded, linesDeleted \
     FROM scored_commits GROUP BY commitHash";

/// Lines grouped as finely as the source allows: by day, by who wrote them, by
/// which model, and by what file they landed in.
///
/// Grouping by all four at once rather than by one at a time is what lets the
/// record answer questions nobody asked while collecting — lines a given model
/// wrote in a given language, say. The row count stays small because the grouping
/// keys are few in practice: a handful of models across a few dozen extensions.
const GENERATED_QUERY: &str = "SELECT date(createdAt / 1000, 'unixepoch', 'localtime') AS day, \
     source, COALESCE(NULLIF(model, ''), '') AS model, \
     COALESCE(fileExtension, '') AS extension, COUNT(*) AS lines \
     FROM ai_code_hashes GROUP BY day, source, model, extension";

const EVENT_QUERY: &str = "SELECT createdAt / 1000 AS second, \
     date(createdAt / 1000, 'unixepoch', 'localtime') AS day, \
     COALESCE(fileExtension, '') AS extension, COUNT(*) AS weight \
     FROM ai_code_hashes GROUP BY second, extension ORDER BY second";

const REQUEST_QUERY: &str = "SELECT date(createdAt / 1000, 'unixepoch', 'localtime') AS day, \
     COUNT(DISTINCT requestId) AS requests \
     FROM ai_code_hashes WHERE requestId IS NOT NULL GROUP BY day";

pub fn database_path(home: &Path) -> PathBuf {
    home.join(".cursor")
        .join("ai-tracking")
        .join("ai-code-tracking.db")
}

pub fn read(
    path: &Path,
    idle_timeout_seconds: i64,
) -> Result<BTreeMap<String, DayBucket>, CollectError> {
    if !path.exists() {
        return Err(CollectError::NotFound {
            what: "Cursor AI tracking database",
            path: path.to_path_buf(),
        });
    }

    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|error| CollectError::Unreadable {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    // This database belongs to the editor, which writes to it continuously and
    // keeps it on a rollback journal rather than a write-ahead log, so a write in
    // progress locks readers out entirely. Waiting is the right answer: the editor
    // is the owner and its commits are short, while a collection that gives up
    // loses the run. The default would be five seconds, which measurement showed
    // is close enough to real contention to be reached.
    connection
        .busy_timeout(LOCK_WAIT)
        .map_err(|error| CollectError::Unreadable {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    let mut days = BTreeMap::new();
    read_committed(&connection, &mut days)?;
    read_generated(&connection, &mut days)?;
    read_worked_time(&connection, idle_timeout_seconds, &mut days)?;
    read_requests(&connection, &mut days)?;
    Ok(days)
}

fn read_committed(
    connection: &Connection,
    days: &mut BTreeMap<String, DayBucket>,
) -> Result<(), CollectError> {
    let mut statement = connection
        .prepare(COMMITTED_QUERY)
        .map_err(|error| schema_error(SCORED_COMMITS, error))?;

    let rows = statement
        .query_map([], |row| {
            let mut counts = [0_i64; 10];
            for (index, slot) in counts.iter_mut().enumerate() {
                *slot = row.get::<_, Option<i64>>(index + 1)?.unwrap_or_default();
            }
            Ok((row.get::<_, Option<String>>(0)?, counts))
        })
        .map_err(|error| schema_error(SCORED_COMMITS, error))?;

    for row in rows {
        let (commit_date, counts) = row.map_err(|error| schema_error(SCORED_COMMITS, error))?;
        let Some(date) = commit_date.as_deref().and_then(commit_day) else {
            continue;
        };

        let tab_added = positive(counts[0]);
        let tab_deleted = positive(counts[1]);
        let agent_added = positive(counts[2]);
        let agent_deleted = positive(counts[3]);
        let human_added = positive(counts[4]);
        let human_deleted = positive(counts[5]);
        let blank_added = positive(counts[6]);
        let blank_deleted = positive(counts[7]);
        let named_added = tab_added + agent_added + human_added + blank_added;
        let named_deleted = tab_deleted + agent_deleted + human_deleted + blank_deleted;

        let bucket = days
            .entry(date.clone())
            .or_insert_with(|| DayBucket::new(date));
        bucket.commits.tab_added += tab_added;
        bucket.commits.tab_deleted += tab_deleted;
        bucket.commits.agent_added += agent_added;
        bucket.commits.agent_deleted += agent_deleted;
        bucket.commits.human_added += human_added;
        bucket.commits.human_deleted += human_deleted;
        bucket.commits.blank_added += blank_added;
        bucket.commits.blank_deleted += blank_deleted;
        bucket.commits.unattributed_added += positive(counts[8]).saturating_sub(named_added);
        bucket.commits.unattributed_deleted += positive(counts[9]).saturating_sub(named_deleted);
    }

    Ok(())
}

fn read_generated(
    connection: &Connection,
    days: &mut BTreeMap<String, DayBucket>,
) -> Result<(), CollectError> {
    let mut statement = connection
        .prepare(GENERATED_QUERY)
        .map_err(|error| schema_error(CODE_HASHES, error))?;

    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|error| schema_error(CODE_HASHES, error))?;

    for row in rows {
        let (day, source, model, extension, lines) =
            row.map_err(|error| schema_error(CODE_HASHES, error))?;
        let Some(date) = day else {
            continue;
        };
        let bucket = days
            .entry(date.clone())
            .or_insert_with(|| DayBucket::new(date));

        // A person's lines carry no model, so the model column is dropped for
        // them rather than recorded as belonging to whatever was loaded.
        let (author, model) = if source == "human" {
            (Author::Human, String::new())
        } else {
            (Author::Agent, model)
        };
        bucket.add_lines(
            language::from_extension(&extension),
            author,
            &model,
            positive(lines),
            0,
        );
    }

    Ok(())
}

fn read_worked_time(
    connection: &Connection,
    idle_timeout_seconds: i64,
    days: &mut BTreeMap<String, DayBucket>,
) -> Result<(), CollectError> {
    let moments = read_moments(connection)?;

    for (date, bucket) in sessions::accumulate(&moments, idle_timeout_seconds) {
        *days
            .entry(date.clone())
            .or_insert_with(|| DayBucket::new(date))
            .measure_mut(MEASURE_AGENT) = bucket;
    }

    Ok(())
}

fn read_moments(connection: &Connection) -> Result<Vec<Event>, CollectError> {
    let mut statement = connection
        .prepare(EVENT_QUERY)
        .map_err(|error| schema_error(CODE_HASHES, error))?;

    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|error| schema_error(CODE_HASHES, error))?;

    let mut moments = Vec::<Event>::new();
    for row in rows {
        let (second, day, extension, weight) =
            row.map_err(|error| schema_error(CODE_HASHES, error))?;
        let Some(day) = day else {
            continue;
        };
        let language = crate::language::from_extension(&extension);

        match moments.last_mut() {
            Some(moment) if moment.second == second => {
                moment.languages.push((language, positive(weight)));
            }
            _ => moments.push(Event {
                second,
                day,
                languages: vec![(language, positive(weight))],
            }),
        }
    }

    for moment in &mut moments {
        moment
            .languages
            .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    }

    Ok(moments)
}

fn read_requests(
    connection: &Connection,
    days: &mut BTreeMap<String, DayBucket>,
) -> Result<(), CollectError> {
    let mut statement = connection
        .prepare(REQUEST_QUERY)
        .map_err(|error| schema_error(CODE_HASHES, error))?;

    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| schema_error(CODE_HASHES, error))?;

    for row in rows {
        let (day, requests) = row.map_err(|error| schema_error(CODE_HASHES, error))?;
        let Some(day) = day else {
            continue;
        };
        days.entry(day.clone())
            .or_insert_with(|| DayBucket::new(day))
            .requests += u32::try_from(requests).unwrap_or(u32::MAX);
    }

    Ok(())
}

pub fn commit_day(commit_date: &str) -> Option<String> {
    let mut fields = commit_date.split_whitespace().skip(1);
    let month = month_number(fields.next()?)?;
    let day = fields.next()?.parse::<u32>().ok()?;
    let year = fields.nth(1)?.parse::<u32>().ok()?;
    if !(1..=31).contains(&day) || year < 1970 {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

fn month_number(name: &str) -> Option<u32> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    MONTHS
        .iter()
        .position(|month| month.eq_ignore_ascii_case(name))
        .map(|index| index as u32 + 1)
}

fn positive(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

/// Sorts a failed query into the two things it can mean.
///
/// Losing the lock race and reading a database whose shape has changed both
/// arrive here as one error type, and they ask for opposite responses: waiting
/// again later, or changing this code. Reporting a lock as a schema problem sends
/// the reader looking for a column that was never missing.
fn schema_error(source: &'static str, error: rusqlite::Error) -> CollectError {
    if let rusqlite::Error::SqliteFailure(failure, _) = &error {
        if matches!(
            failure.code,
            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
        ) {
            return CollectError::Busy {
                what: source,
                waited: LOCK_WAIT,
            };
        }
    }
    CollectError::UnexpectedSchema {
        source,
        message: error.to_string(),
    }
}
