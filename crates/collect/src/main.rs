use std::{env, error::Error, path::PathBuf, process};

use github_personal_stats_collect::{
    DEFAULT_IDLE_TIMEOUT_MINUTES, Settings, collect, machine, preferences::Preferences, sink,
};
use github_personal_stats_core::summarise_activity;

/// Named relative to the state directory, not to whoever is calling. The record
/// is data the app manages, like the token and the pulse journal, and a default
/// that moved with the working directory made every command disagree about where
/// it lived.
const DEFAULT_SNAPSHOT: &str = "record";

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "help" | "--help" | "-h"))
    {
        println!("{}", usage());
        return Ok(());
    }

    let home = match option(&args, "--home") {
        Some(value) => PathBuf::from(value),
        None => env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or("HOME is not set, so there is no place to look for local records")?,
    };

    let state_dir = option(&args, "--state")
        .map(PathBuf::from)
        .unwrap_or_else(|| machine::state_directory(&home));
    // The configuration lives beside the machine identity, so finding it needs
    // only the state directory, which is settled before anything else is read.
    let prefs = Preferences::load(&state_dir);

    let settings = Settings {
        snapshot: chosen(&args, &prefs, "--output")
            .map(PathBuf::from)
            .unwrap_or_else(|| state_dir.join(DEFAULT_SNAPSHOT)),
        idle_timeout_seconds: idle_timeout(&args, &prefs)?,
        state_dir,
        home,
    };

    let snapshot = collect(&settings)?;
    let sink = sink::choose(
        chosen(&args, &prefs, "--sink").as_deref(),
        &settings.snapshot,
        chosen(&args, &prefs, "--repo").as_deref(),
        chosen(&args, &prefs, "--origin").as_deref(),
        chosen(&args, &prefs, "--branch").as_deref(),
        !(args.iter().any(|arg| arg == "--no-push") || prefs.switch("no-push")),
    )?;
    let written = sink.publish(&snapshot)?;

    let totals = summarise_activity(&snapshot.days);
    println!(
        "{} days recorded as {} through {}",
        snapshot.days.len(),
        snapshot.machine,
        written.display()
    );
    println!(
        "{} hrs {} mins of code changing by agents, across {} sessions",
        totals.agent.seconds / 3_600,
        (totals.agent.seconds % 3_600) / 60,
        totals.agent.sessions
    );
    if totals.editor.seconds > 0 {
        println!(
            "{} hrs {} mins in the editor, across {} sessions",
            totals.editor.seconds / 3_600,
            (totals.editor.seconds % 3_600) / 60,
            totals.editor.sessions
        );
    }
    println!(
        "{} lines committed, {} of them attributable, {}% of those written by AI",
        totals.committed.added(),
        totals.committed.attributed_added(),
        totals.committed.ai_share_basis_points() / 100
    );
    println!(
        "{} lines generated in the editor, {}% of them by AI",
        totals.generated.total(),
        totals.generated.ai_share_basis_points() / 100
    );

    Ok(())
}

fn idle_timeout(args: &[String], prefs: &Preferences) -> Result<i64, Box<dyn Error>> {
    let minutes = match chosen(args, prefs, "--idle-timeout") {
        Some(value) => value
            .parse::<i64>()
            .map_err(|_| format!("--idle-timeout wants whole minutes, not {value:?}"))?,
        None => DEFAULT_IDLE_TIMEOUT_MINUTES,
    };
    if !(1..=180).contains(&minutes) {
        return Err(format!("--idle-timeout wants 1 to 180 minutes, not {minutes}").into());
    }
    Ok(minutes * 60)
}

/// What a flag was set to, on the command line if given there, in the
/// configuration otherwise.
fn chosen(args: &[String], prefs: &Preferences, name: &str) -> Option<String> {
    option(args, name).or_else(|| prefs.flag(name).map(str::to_owned))
}

fn option(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn usage() -> String {
    format!(
        "Reads what your editor and AI agents already recorded on this machine into an
activity snapshot, which the renderer draws without asking any service for anything.

Usage:
  github-personal-stats-collect [options]

Options:
  --output <path>          Directory to grow the record in (default: <state>/{DEFAULT_SNAPSHOT})
  --sink <file|git>        Where the record goes (default: file)
  --repo <dir>             Where to keep the storage checkout, for --sink git
  --origin <url>           Git remote to clone the checkout from, and push to
  --branch <name>          Branch to push to (default: master)
  --no-push                Commit locally without pushing
  --idle-timeout <minutes> Gap that ends a working stretch (default: {DEFAULT_IDLE_TIMEOUT_MINUTES})
  --state <dir>            Where the machine identity, token and pulse journal live
                           (default: XDG state directory)
  --home <dir>             Where to look for local records (default: HOME)
  help, --help, -h         Print this text

What is read:
  Cursor keeps a record of the code it wrote at
  ~/.cursor/ai-tracking/ai-code-tracking.db. Committed lines come from its own
  per-commit scoring; generated lines, models, languages, and agent time come
  from the timestamps on the code it produced. Editor time comes from the pulse
  journal that editor plugins write through the daemon.

How the record is laid out:
  One directory per machine, one file per day inside it, and a manifest holding
  the day index and the lifetime totals:

    snapshots/m-1a2b3c4d/manifest.json
    snapshots/m-1a2b3c4d/2026-08-25.json
    snapshots/m-1a2b3c4d/2026-08-26.json

  A day is written once and afterwards only ever replaced by a fuller reading of
  that same day, so a run touches the day it learned something about and leaves
  the rest alone. That is what lets a reader fetch a window instead of the whole
  history, and what makes each commit say which day it recorded.

Publishing:
  --sink git commits the record into a git repository and pushes. Each machine
  writes only inside its own directory, so several machines share one repository
  with nothing to merge: whoever renders the cards adds the days up. The checkout
  is cloned from --origin if it is not there yet, and is brought up to date
  before each commit. Any git remote will do, on the public internet or not; it
  shells out to git, so a private repository works with the credentials you
  already have. A run that changes nothing commits nothing.

Configuration:
  Options can be written once in <state>/config instead of repeated, as
  `name = value` lines using the option's own name without the dashes:

    sink = git
    repo = /path/to/storage/checkout
    origin = git@example.com:you/personal-stats-data.git

  A flag on the command line overrides the file.

What is written:
  Counts and seconds only. No prompt text, no file paths, no project names, no
  repository names, and no host name ever enters the record. The machine is
  named by a random local identifier so two machines can be told apart without
  saying anything about either.

Growing a history:
  Cursor keeps roughly thirty days of detail, and every run reads it afresh, so a
  run made today knows nothing about a day three months ago. The record is
  therefore never replaced by a run; each day is merged into it, keeping whichever
  reading of that day saw more. A day that has aged out of Cursor's store keeps
  the figures it was published with, so running this regularly accumulates a
  history longer than any source it was read from, and running it twice in an hour
  changes nothing.

  The cost of that rule is that a correction downwards cannot land on a day
  already published, because the larger figure outranks it. Delete the day's file
  to have it collected again from scratch."
    )
}
