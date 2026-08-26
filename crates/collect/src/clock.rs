use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;

use crate::error::CollectError;

const SECONDS_PER_DAY: i64 = 86_400;
const SECONDS_PER_HOUR: i64 = 3_600;

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

pub fn utc_timestamp(seconds_since_epoch: i64) -> String {
    let days = seconds_since_epoch.div_euclid(SECONDS_PER_DAY);
    let rest = seconds_since_epoch.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    let hour = rest / 3_600;
    let minute = (rest % 3_600) / 60;
    let second = rest % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Turns instants into the local dates a day of work is filed under.
///
/// The editor's own record is bucketed by SQLite's idea of local time, so this
/// asks SQLite too rather than reimplementing a second idea of it. Two sources
/// disagreeing about which day a few minutes either side of midnight belongs to
/// would be a difference nobody could see and nobody could explain.
///
/// Answers are cached by the hour, which is the coarsest thing a daylight saving
/// change can happen on, so a record of thousands of moments costs a few dozen
/// queries.
pub struct LocalCalendar {
    connection: Connection,
    known: BTreeMap<i64, String>,
}

impl LocalCalendar {
    pub fn new() -> Result<Self, CollectError> {
        Ok(Self {
            connection: Connection::open_in_memory().map_err(|error| {
                CollectError::UnexpectedSchema {
                    source: "the local calendar",
                    message: error.to_string(),
                }
            })?,
            known: BTreeMap::new(),
        })
    }

    pub fn day(&mut self, seconds_since_epoch: i64) -> Result<String, CollectError> {
        let hour = seconds_since_epoch.div_euclid(SECONDS_PER_HOUR);
        if let Some(day) = self.known.get(&hour) {
            return Ok(day.clone());
        }
        let day: String = self
            .connection
            .query_row(
                "SELECT date(?1, 'unixepoch', 'localtime')",
                [seconds_since_epoch],
                |row| row.get(0),
            )
            .map_err(|error| CollectError::UnexpectedSchema {
                source: "the local calendar",
                message: error.to_string(),
            })?;
        self.known.insert(hour, day.clone());
        Ok(day)
    }
}

/// Reads the timestamps agent transcripts are written with: an ISO 8601 instant
/// in UTC, to whatever precision the writer felt like. Anything finer than a
/// second is dropped, since a day is the finest thing it will be filed under.
pub fn instant_from_iso8601(text: &str) -> Option<i64> {
    let text = text.trim();
    let (date, rest) = text.split_once('T')?;
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;

    let clock = rest
        .trim_end_matches('Z')
        .split_once('.')
        .map(|(before, _)| before)
        .unwrap_or_else(|| rest.trim_end_matches('Z'));
    let mut parts = clock.split(':');
    let hour: i64 = parts.next()?.parse().ok()?;
    let minute: i64 = parts.next()?.parse().ok()?;
    let second: i64 = parts.next().unwrap_or("0").parse().ok()?;

    Some(
        days_from_civil(year, month, day) * SECONDS_PER_DAY
            + hour * SECONDS_PER_HOUR
            + minute * 60
            + second,
    )
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let shifted_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (year + i64::from(month <= 2), month, day)
}
