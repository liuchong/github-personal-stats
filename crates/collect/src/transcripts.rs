//! Tokens read from the records terminal agents leave behind.
//!
//! Two agents, two formats, one question: how many tokens went to which model on
//! which day. Both keep a file per session under a directory of their own, and
//! both write one JSON object per line, so both are read the same way — a line at
//! a time, parsing only the lines that could carry what is wanted.
//!
//! Reading the files rather than asking the agents to report gives the record its
//! history: a hook installed today knows nothing about last month, and these
//! files go back as far as the agent has been used. It also means nothing has to
//! be installed for the figures to appear.
//!
//! Nothing but a day, a model name and a count of tokens is taken. The files also
//! hold working directories, prompts, tool output and the contents of edited
//! files; none of that is read, and the lines carrying it are skipped before they
//! are parsed.

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use github_personal_stats_core::{DayBucket, TokenUsage};
use serde_json::Value;

use crate::{clock::LocalCalendar, error::CollectError};

/// Where each agent keeps its sessions, relative to the home directory.
const CODEX_SESSIONS: [&str; 2] = [".codex", "sessions"];
const CLAUDE_SESSIONS: [&str; 2] = [".claude", "projects"];

pub fn codex_path(home: &Path) -> PathBuf {
    home.join(CODEX_SESSIONS[0]).join(CODEX_SESSIONS[1])
}

pub fn claude_path(home: &Path) -> PathBuf {
    home.join(CLAUDE_SESSIONS[0]).join(CLAUDE_SESSIONS[1])
}

/// Reads whichever of the two records is present, adding to `days`.
///
/// A missing directory is not an error: it means that agent was never used here,
/// and a machine that only ever ran one of them should not have to say so.
pub fn read(home: &Path, days: &mut BTreeMap<String, DayBucket>) -> Result<(), CollectError> {
    let mut calendar = LocalCalendar::new()?;
    read_codex(&codex_path(home), &mut calendar, days)?;
    read_claude(&claude_path(home), &mut calendar, days)?;
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
) -> Result<(), CollectError> {
    for path in transcripts(root)? {
        let mut model = String::new();
        for line in lines(&path)? {
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
) -> Result<(), CollectError> {
    for path in transcripts(root)? {
        for line in lines(&path)? {
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

/// A transcript's lines, one at a time.
///
/// These files reach hundreds of megabytes and a single line can hold an entire
/// edited file, so they are never read whole.
fn lines(path: &Path) -> Result<impl Iterator<Item = String> + use<>, CollectError> {
    let file = File::open(path).map_err(|error| CollectError::Unreadable {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    Ok(BufReader::new(file).lines().map_while(Result::ok))
}
