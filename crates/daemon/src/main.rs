use std::{env, error::Error, path::PathBuf, process, sync::Arc, thread};

use github_personal_stats_collect::{DEFAULT_IDLE_TIMEOUT_MINUTES, Settings, machine};
use github_personal_stats_daemon::{DEFAULT_ADDRESS, DEFAULT_INTERVAL_MINUTES, Daemon, token};

const DEFAULT_SNAPSHOT: &str = "activity.json";

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
            .ok_or("HOME is not set, so there is no place to keep local records")?,
    };

    let settings = Settings {
        state_dir: option(&args, "--state")
            .map(PathBuf::from)
            .unwrap_or_else(|| machine::state_directory(&home)),
        snapshot: option(&args, "--output")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SNAPSHOT)),
        idle_timeout_seconds: whole(
            &args,
            "--idle-timeout",
            DEFAULT_IDLE_TIMEOUT_MINUTES,
            1,
            180,
        )? * 60,
        home,
    };

    match args.first().map(String::as_str) {
        Some("token") => {
            let token = token::read_or_mint(&settings.state_dir)?;
            println!("{}", token::path(&settings.state_dir).display());
            println!("{token}");
            Ok(())
        }
        Some("serve") | None => serve(&args, settings),
        Some(other) if other.starts_with('-') => serve(&args, settings),
        Some(other) => Err(format!("no command called {other:?}; try help").into()),
    }
}

fn serve(args: &[String], settings: Settings) -> Result<(), Box<dyn Error>> {
    let address = option(args, "--addr").unwrap_or_else(|| DEFAULT_ADDRESS.to_owned());
    let interval = whole(
        args,
        "--interval",
        DEFAULT_INTERVAL_MINUTES as i64,
        1,
        24 * 60,
    )? as u64;

    let daemon = Arc::new(Daemon::new(settings)?);
    let listener = daemon.listen(&address)?;

    println!(
        "listening on http://{address}\npanel http://{address}/?token={}\nsnapshot {}\nrebuilding every {interval} minutes",
        daemon.token(),
        daemon.settings().snapshot.display()
    );

    let timer = Arc::clone(&daemon);
    thread::spawn(move || timer.rebuild_on_schedule(interval));

    daemon.serve(&listener);
    Ok(())
}

fn option(args: &[String], name: &str) -> Option<String> {
    let mut iterator = args.iter();
    while let Some(argument) = iterator.next() {
        if argument == name {
            return iterator.next().cloned();
        }
        if let Some(value) = argument.strip_prefix(&format!("{name}=")) {
            return Some(value.to_owned());
        }
    }
    None
}

fn whole(
    args: &[String],
    name: &str,
    fallback: i64,
    least: i64,
    most: i64,
) -> Result<i64, Box<dyn Error>> {
    let value = match option(args, name) {
        Some(text) => text
            .parse::<i64>()
            .map_err(|_| format!("{name} wants a whole number of minutes, not {text:?}"))?,
        None => fallback,
    };
    if !(least..=most).contains(&value) {
        return Err(format!("{name} wants between {least} and {most} minutes, not {value}").into());
    }
    Ok(value)
}

fn usage() -> String {
    format!(
        "github-personal-stats-daemon — receive editor pulses, keep a snapshot, serve a panel

USAGE
    github-personal-stats-daemon [serve] [options]
    github-personal-stats-daemon token
    github-personal-stats-daemon help

COMMANDS
    serve    Listen for pulses and rebuild the snapshot on a timer. The default.
    token    Print where the shared secret lives, and what it is, for a plugin
             that cannot read the file itself.

OPTIONS
    --addr <host:port>    Where to listen. Loopback only. Default {DEFAULT_ADDRESS}.
    --interval <minutes>  How often to rebuild the snapshot. Default {DEFAULT_INTERVAL_MINUTES}.
    --idle-timeout <min>  A gap longer than this ends a session rather than
                          counting as time worked. Default {DEFAULT_IDLE_TIMEOUT_MINUTES}.
    --output <path>       Where to write the snapshot. Default {DEFAULT_SNAPSHOT}.
    --state <path>        Where the machine id, the token and the pulse journal
                          live. Defaults to the XDG state directory.
    --home <path>         Where to look for local editor records. Defaults to HOME.

ENDPOINTS
    GET  /v1/health       Whether the daemon is up. No token needed.
    POST /v1/pulses       Report pulses. Body {{\"editor\":\"vscode\",\"pulses\":[...]}}.
    POST /v1/collect      Rebuild the snapshot now.
    GET  /v1/summary      The current totals as JSON.
    GET  /                The panel, as a page.

    Every endpoint but health wants the token, as an Authorization bearer header
    or a token query parameter."
    )
}
