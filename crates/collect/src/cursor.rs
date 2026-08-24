use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use github_personal_stats_core::DayBucket;
use rusqlite::{Connection, OpenFlags};

use crate::error::CollectError;

const SCORED_COMMITS: &str = "scored_commits";
const CODE_HASHES: &str = "ai_code_hashes";

const COMMITTED_QUERY: &str = "SELECT commitDate, \
     tabLinesAdded, tabLinesDeleted, \
     composerLinesAdded, composerLinesDeleted, \
     humanLinesAdded, humanLinesDeleted, \
     blankLinesAdded, blankLinesDeleted, \
     linesAdded, linesDeleted \
     FROM scored_commits GROUP BY commitHash";

const GENERATED_QUERY: &str = "SELECT date(createdAt / 1000, 'unixepoch', 'localtime') AS day, \
     source, COALESCE(NULLIF(model, ''), 'unknown') AS model, COUNT(*) AS lines \
     FROM ai_code_hashes GROUP BY day, source, model";

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
        bucket.committed.tab_added += tab_added;
        bucket.committed.tab_deleted += tab_deleted;
        bucket.committed.agent_added += agent_added;
        bucket.committed.agent_deleted += agent_deleted;
        bucket.committed.human_added += human_added;
        bucket.committed.human_deleted += human_deleted;
        bucket.committed.blank_added += blank_added;
        bucket.committed.blank_deleted += blank_deleted;
        bucket.committed.unattributed_added += positive(counts[8]).saturating_sub(named_added);
        bucket.committed.unattributed_deleted += positive(counts[9]).saturating_sub(named_deleted);
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
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|error| schema_error(CODE_HASHES, error))?;

    for row in rows {
        let (day, source, model, lines) = row.map_err(|error| schema_error(CODE_HASHES, error))?;
        let Some(date) = day else {
            continue;
        };
        let lines = positive(lines);
        let bucket = days
            .entry(date.clone())
            .or_insert_with(|| DayBucket::new(date));

        if source == "human" {
            bucket.generated.human += lines;
        } else {
            *bucket.generated.by_model.entry(model).or_default() += lines;
        }
    }

    Ok(())
}

struct Moment {
    second: i64,
    day: String,
    extensions: Vec<(&'static str, u64)>,
}

fn read_worked_time(
    connection: &Connection,
    idle_timeout_seconds: i64,
    days: &mut BTreeMap<String, DayBucket>,
) -> Result<(), CollectError> {
    let moments = read_moments(connection)?;
    let mut previous: Option<&Moment> = None;

    for moment in &moments {
        let starts_session = match previous {
            None => true,
            Some(earlier) => {
                let gap = moment.second - earlier.second;
                if gap > 0 && gap <= idle_timeout_seconds {
                    spend(days, earlier, positive(gap));
                    false
                } else {
                    gap > idle_timeout_seconds
                }
            }
        };

        if starts_session {
            days.entry(moment.day.clone())
                .or_insert_with(|| DayBucket::new(moment.day.clone()))
                .sessions += 1;
        }

        previous = Some(moment);
    }

    Ok(())
}

fn read_moments(connection: &Connection) -> Result<Vec<Moment>, CollectError> {
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

    let mut moments = Vec::<Moment>::new();
    for row in rows {
        let (second, day, extension, weight) =
            row.map_err(|error| schema_error(CODE_HASHES, error))?;
        let Some(day) = day else {
            continue;
        };
        let language = crate::language::from_extension(&extension);

        match moments.last_mut() {
            Some(moment) if moment.second == second => {
                moment.extensions.push((language, positive(weight)));
            }
            _ => moments.push(Moment {
                second,
                day,
                extensions: vec![(language, positive(weight))],
            }),
        }
    }

    for moment in &mut moments {
        moment
            .extensions
            .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    }

    Ok(moments)
}

fn spend(days: &mut BTreeMap<String, DayBucket>, moment: &Moment, seconds: u64) {
    let bucket = days
        .entry(moment.day.clone())
        .or_insert_with(|| DayBucket::new(moment.day.clone()));
    bucket.seconds += seconds;

    let weight: u64 = moment.extensions.iter().map(|(_, weight)| weight).sum();
    if weight == 0 {
        return;
    }

    let mut spent = 0;
    for (language, share) in &moment.extensions {
        let slice = seconds * share / weight;
        *bucket.languages.entry(language.to_string()).or_default() += slice;
        spent += slice;
    }

    if let Some((language, _)) = moment.extensions.first() {
        *bucket.languages.entry(language.to_string()).or_default() += seconds - spent;
    }
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

fn schema_error(source: &'static str, error: rusqlite::Error) -> CollectError {
    CollectError::UnexpectedSchema {
        source,
        message: error.to_string(),
    }
}
