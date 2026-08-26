//! What terminal agents leave behind: when they were working, and what it cost.
//!
//! Two agents, two formats, the same two questions. Both keep a file per session
//! under a directory of their own, and both write one JSON object per line, so
//! both are read the same way — a line at a time, parsing only the lines that
//! could carry what is wanted.
//!
//! Reading these matters more than it looks. Measurement of one month's record
//! found eight thousand minutes in which the editor saw an agent write code and
//! thirteen thousand further minutes in which only a terminal agent was working:
//! more work happening outside the editor's view than inside it. A measure built
//! on the editor alone does not merely undercount those hours, it fills them in
//! by interpolation, which is a guess where there is evidence to be had.
//!
//! Reading the files rather than asking the agents to report also gives the
//! record its history: a hook installed today knows nothing about last month,
//! and these files go back as far as the agent has been used.
//!
//! Nothing but a timestamp, a model name and a count of tokens is taken. The
//! files also hold working directories, prompts, shell commands, tool output and
//! the contents of edited files; none of that is read. That is also why a moment
//! from here carries no language: the only place these transcripts name a file is
//! inside the text of a shell command, and reading that to guess an extension
//! would mean reading the one thing this promised not to.

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use github_personal_stats_core::{Author, DayBucket, TokenUsage, UNKNOWN_LANGUAGE};
use serde_json::Value;

use crate::{
    clock::LocalCalendar,
    error::CollectError,
    sessions::{Event, Part},
};

/// The longest line worth looking at.
///
/// A transcript line can hold an entire edited file; the largest measured in this
/// record was thirty-seven megabytes inside a file of five gigabytes. Nothing
/// this module wants is ever in such a line — a timestamp, a model name and a
/// handful of counts all sit within the first few hundred bytes of the objects
/// that carry them — so a line beyond this is skipped without being held. The
/// bound is what keeps the collector's memory a property of this constant rather
/// than of whatever the largest file an agent ever touched happened to be.
const LINE_LIMIT: usize = 64 * 1024;

/// Where each agent keeps its sessions, relative to the home directory.
const CODEX_SESSIONS: [&str; 2] = [".codex", "sessions"];
const CLAUDE_SESSIONS: [&str; 2] = [".claude", "projects"];

pub fn codex_path(home: &Path) -> PathBuf {
    home.join(CODEX_SESSIONS[0]).join(CODEX_SESSIONS[1])
}

pub fn claude_path(home: &Path) -> PathBuf {
    home.join(CLAUDE_SESSIONS[0]).join(CLAUDE_SESSIONS[1])
}

/// Reads whichever of the two records is present, adding tokens to `days` and
/// returning the moments the sessions were seen at.
///
/// A missing directory is not an error: it means that agent was never used here,
/// and a machine that only ever ran one of them should not have to say so.
pub fn read(
    home: &Path,
    days: &mut BTreeMap<String, DayBucket>,
) -> Result<Vec<Event>, CollectError> {
    let mut calendar = LocalCalendar::new()?;
    let mut moments = Vec::new();
    read_codex(&codex_path(home), &mut calendar, days, &mut moments)?;
    read_claude(&claude_path(home), &mut calendar, days, &mut moments)?;
    moments.sort_by_key(|moment| moment.second);
    Ok(moments)
}

/// Notes that an agent was working at this instant.
///
/// Every timestamped line is evidence of that, whatever the line was about, which
/// is the whole of what a moment claims. What it does not claim is a file: these
/// carry no language, and the time they account for is filed under the unnamed
/// one rather than being spread over the languages the editor happened to see.
/// Naming it would put hours against a language on no evidence at all.
fn observed(
    calendar: &mut LocalCalendar,
    stamp: &str,
    model: &str,
    moments: &mut Vec<Event>,
) -> Result<(), CollectError> {
    let Some(instant) = crate::clock::instant_from_iso8601(stamp) else {
        return Ok(());
    };
    let day = calendar.day(instant)?;
    moments.push(Event {
        second: instant,
        day,
        languages: vec![Part::new(UNKNOWN_LANGUAGE, Some(Author::Agent), 1).by(model_name(model))],
    });
    Ok(())
}

/// Codex counts tokens twice over: a running total for the session and the cost
/// of the last request. They agree — the totals are the running sum of the
/// requests — and the per-request figure is the one taken, because a session that
/// runs past midnight spends on both days and only the per-request figure can say
/// how much on each.
fn read_codex(
    root: &Path,
    calendar: &mut LocalCalendar,
    days: &mut BTreeMap<String, DayBucket>,
    moments: &mut Vec<Event>,
) -> Result<(), CollectError> {
    for path in transcripts(root)? {
        let mut model = String::new();
        for line in lines(&path)? {
            // Every timestamped line says the session was live then, which is
            // what a moment is; only some of them say what it cost. The cheap
            // test comes first so that the great majority of lines are neither
            // parsed nor held.
            let Some(stamp) = stamp_in(&line) else {
                continue;
            };
            observed(calendar, &stamp, &model, moments)?;

            if !line.contains("\"token_count\"") && !line.contains("\"turn_context\"") {
                continue;
            }
            let Ok(event) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let payload = &event["payload"];

            if event["type"] == "turn_context" {
                if let Some(named) = payload["model"].as_str() {
                    model = named.to_owned();
                }
            }
            if payload["type"] != "token_count" {
                continue;
            }
            let Some(stamp) = event["timestamp"].as_str() else {
                continue;
            };
            let spent = &payload["info"]["last_token_usage"];
            // Codex counts cached input inside its input figure, and reasoning
            // inside its output. The record keeps the cache apart, since tokens a
            // cache served were not paid for at the same rate.
            let input = number(&spent["input_tokens"]);
            let cached = number(&spent["cached_input_tokens"]);
            let usage = TokenUsage {
                input: input.saturating_sub(cached),
                output: number(&spent["output_tokens"]),
                cached,
            };
            spend(days, calendar, stamp, &model, usage)?;
        }
    }
    Ok(())
}

/// Claude Code reports usage on each message it receives, which is already the
/// cost of that one request, so the figures are summed as they are read.
fn read_claude(
    root: &Path,
    calendar: &mut LocalCalendar,
    days: &mut BTreeMap<String, DayBucket>,
    moments: &mut Vec<Event>,
) -> Result<(), CollectError> {
    for path in transcripts(root)? {
        for line in lines(&path)? {
            let Some(stamp) = stamp_in(&line) else {
                continue;
            };
            // Claude names the model on the messages that report usage, so a
            // moment from any other line has none to give. That is the truth
            // about it rather than a gap worth filling from a neighbour.
            let named = model_of(&line);
            observed(
                calendar,
                &stamp,
                named.as_deref().unwrap_or_default(),
                moments,
            )?;

            if !line.contains("\"usage\"") {
                continue;
            }
            let Ok(event) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let message = &event["message"];
            let spent = &message["usage"];
            if !spent.is_object() {
                continue;
            }
            let Some(stamp) = event["timestamp"].as_str() else {
                continue;
            };
            // Writing to the cache is charged; reading from it is not, so only
            // the read counts as cached.
            let usage = TokenUsage {
                input: number(&spent["input_tokens"])
                    + number(&spent["cache_creation_input_tokens"]),
                output: number(&spent["output_tokens"]),
                cached: number(&spent["cache_read_input_tokens"]),
            };
            let model = message["model"].as_str().unwrap_or_default();
            spend(days, calendar, stamp, model, usage)?;
        }
    }
    Ok(())
}

fn spend(
    days: &mut BTreeMap<String, DayBucket>,
    calendar: &mut LocalCalendar,
    stamp: &str,
    model: &str,
    usage: TokenUsage,
) -> Result<(), CollectError> {
    if usage.total() == 0 {
        return Ok(());
    }
    let Some(instant) = crate::clock::instant_from_iso8601(stamp) else {
        return Ok(());
    };
    let day = calendar.day(instant)?;
    let held = days
        .entry(day.clone())
        .or_insert_with(|| DayBucket::new(day))
        .tokens
        .entry(model_name(model))
        .or_default();
    held.input += usage.input;
    held.output += usage.output;
    held.cached += usage.cached;
    Ok(())
}

/// What to file usage under when the agent did not name a model.
///
/// Some sessions are answered by whatever the router picked and report that as
/// the model, which is the truthful answer to what was asked for even though it
/// is not the name of a model. It is kept as it was reported; inventing a name
/// would be worse than an unhelpful one.
fn model_name(reported: &str) -> String {
    if reported.is_empty() {
        "unnamed".to_owned()
    } else {
        reported.to_owned()
    }
}

fn number(value: &Value) -> u64 {
    value.as_u64().unwrap_or_default()
}

/// The timestamp of a line, cut out rather than parsed.
///
/// Every line of these files carries one and almost none of them carry anything
/// else this module wants, so the common case has to be cheap: parsing each line
/// into a document to reach one field near its front would be the difference
/// between reading these records and not being able to afford to.
fn stamp_in(line: &str) -> Option<String> {
    field(line, "timestamp")
}

fn model_of(line: &str) -> Option<String> {
    field(line, "model")
}

/// Reads `"name":"value"` out of a line, allowing the spacing a writer may add.
fn field(line: &str, name: &str) -> Option<String> {
    let key = format!("\"{name}\"");
    let at = line.find(&key)? + key.len();
    let rest = line[at..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    // A value with an escape in it is not one of these fields, so the first
    // backslash means this was something else with a similar name.
    let end = rest.find('"')?;
    if rest[..end].contains('\\') {
        return None;
    }
    Some(rest[..end].to_owned())
}

/// Every `.jsonl` under a root, at any depth, in a settled order.
///
/// One agent files sessions by date and the other by project, so the depth is
/// not the same between them and neither is worth knowing: a transcript is a
/// transcript wherever it sits.
fn transcripts(root: &Path) -> Result<Vec<PathBuf>, CollectError> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory).map_err(|error| CollectError::Unreadable {
            path: directory.clone(),
            message: error.to_string(),
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| CollectError::Unreadable {
                path: directory.clone(),
                message: error.to_string(),
            })?;
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|kind| kind == "jsonl") {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// A transcript's lines, one at a time, and none of them longer than the limit.
///
/// These files reach gigabytes and a single line can hold an entire edited file,
/// so they are never read whole and no line is held in full either. A line over
/// the limit is drained and dropped: it cannot hold anything wanted, and holding
/// it would make this reader's memory depend on the largest file some agent once
/// touched.
fn lines(path: &Path) -> Result<impl Iterator<Item = String> + use<>, CollectError> {
    let file = File::open(path).map_err(|error| CollectError::Unreadable {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let mut reader = BufReader::new(file);
    let mut held = String::new();
    Ok(std::iter::from_fn(move || {
        loop {
            held.clear();
            let kept = read_bounded(&mut reader, &mut held)?;
            if kept {
                return Some(std::mem::take(&mut held));
            }
        }
    }))
}

/// Reads one line, keeping at most `LINE_LIMIT` of it.
///
/// Returns whether the line was short enough to be worth returning, or `None` at
/// the end of the file. An over-long line is still consumed to its end, since the
/// next line has to start in the right place.
fn read_bounded(reader: &mut impl BufRead, into: &mut String) -> Option<bool> {
    let mut room = LINE_LIMIT;
    let mut overran = false;
    loop {
        let available = match reader.fill_buf() {
            // The end of the file. A last line with no newline after it still
            // counts, and so does an over-long one, which reports itself as
            // dropped rather than as the end.
            Ok([]) => return (!into.is_empty() || overran).then_some(!overran),
            Ok(bytes) => bytes,
            Err(_) => return None,
        };
        let (chunk, done) = match available.iter().position(|byte| *byte == b'\n') {
            Some(at) => (&available[..at], Some(at + 1)),
            None => (available, None),
        };
        let taken = chunk.len();
        if taken > room {
            overran = true;
            into.clear();
            room = 0;
        } else {
            into.push_str(&String::from_utf8_lossy(chunk));
            room -= taken;
        }
        reader.consume(done.unwrap_or(taken));
        if done.is_some() {
            return Some(!overran);
        }
    }
}
