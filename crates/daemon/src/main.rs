use std::{
    env, error::Error, fs, net::TcpStream, path::PathBuf, process, sync::Arc, thread,
    time::Duration,
};

use github_personal_stats_collect::{
    DEFAULT_IDLE_TIMEOUT_MINUTES, Settings, clock, machine,
    preferences::Preferences,
    presence, pulse,
    sink::{self, Sink},
};
use github_personal_stats_core::{parse_activity_snapshot, summarise_activity};
use github_personal_stats_daemon::{
    DEFAULT_ADDRESS, DEFAULT_INTERVAL_MINUTES, Daemon, service, token,
};

/// Named relative to the state directory, not to whoever is calling. The
/// snapshot is data the app manages, like the token and the pulse journal, and a
/// default that moved with the working directory made every command disagree
/// about where it lived.
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
        idle_timeout_seconds: whole(
            &args,
            &prefs,
            "--idle-timeout",
            DEFAULT_IDLE_TIMEOUT_MINUTES,
            1,
            180,
        )? * 60,
        state_dir,
        home,
    };

    match args.first().map(String::as_str) {
        Some("token") => {
            let token = token::read_or_mint(&settings.state_dir)?;
            println!("{}", token::path(&settings.state_dir).display());
            println!("{token}");
            Ok(())
        }
        Some("status") => status(&args, &settings, &prefs),
        Some("install") => install(&args, &settings, &prefs),
        Some("uninstall") => uninstall(),
        Some("serve") | None => serve(&args, settings, &prefs),
        Some(other) if other.starts_with('-') => serve(&args, settings, &prefs),
        Some(other) => Err(format!("no command called {other:?}; try help").into()),
    }
}

fn sink_for(
    args: &[String],
    settings: &Settings,
    prefs: &Preferences,
) -> Result<Box<dyn Sink + Send + Sync>, Box<dyn Error>> {
    Ok(sink::choose(
        chosen(args, prefs, "--sink").as_deref(),
        &settings.snapshot,
        chosen(args, prefs, "--repo").as_deref(),
        chosen(args, prefs, "--origin").as_deref(),
        chosen(args, prefs, "--branch").as_deref(),
        !(args.iter().any(|arg| arg == "--no-push") || prefs.switch("no-push")),
    )?)
}

/// What a flag was set to, on the command line if given there, in the
/// configuration otherwise.
fn chosen(args: &[String], prefs: &Preferences, name: &str) -> Option<String> {
    option(args, name).or_else(|| prefs.flag(name).map(str::to_owned))
}

fn serve(args: &[String], settings: Settings, prefs: &Preferences) -> Result<(), Box<dyn Error>> {
    let address = chosen(args, prefs, "--addr").unwrap_or_else(|| DEFAULT_ADDRESS.to_owned());
    let interval = whole(
        args,
        prefs,
        "--interval",
        DEFAULT_INTERVAL_MINUTES as i64,
        1,
        24 * 60,
    )? as u64;

    let sink = sink_for(args, &settings, prefs)?;
    let publishing = sink.describe();
    let daemon = Arc::new(Daemon::new(settings, sink)?);
    let listener = daemon.listen(&address)?;

    println!(
        "listening on http://{address}\npanel http://{address}/?token={}\npublishing to {publishing}\nrebuilding every {interval} minutes",
        daemon.token(),
    );

    let timer = Arc::clone(&daemon);
    thread::spawn(move || timer.rebuild_on_schedule(interval));

    daemon.serve(&listener);
    Ok(())
}

/// Answers "is this working" without asking anyone to read source or list
/// directories. Each line is a thing that can be separately broken: the daemon
/// running, an editor reporting, and a snapshot reaching its destination. A plugin
/// that is installed but never loaded looks exactly like one that is working,
/// until something says which editors have actually been heard from.
fn status(args: &[String], settings: &Settings, prefs: &Preferences) -> Result<(), Box<dyn Error>> {
    let address = chosen(args, prefs, "--addr").unwrap_or_else(|| DEFAULT_ADDRESS.to_owned());
    let reachable = TcpStream::connect_timeout(
        &address
            .parse()
            .map_err(|_| format!("{address} is not a host and port"))?,
        Duration::from_millis(500),
    )
    .is_ok();

    println!(
        "daemon      {}",
        if reachable {
            format!("listening on {address}")
        } else {
            format!("not listening on {address}")
        }
    );

    let token = token::path(&settings.state_dir);
    println!(
        "token       {}",
        if token.exists() {
            token.display().to_string()
        } else {
            format!("missing at {} — no plugin can report", token.display())
        }
    );

    println!(
        "publishing  {}",
        sink_for(args, settings, prefs)?.describe()
    );

    // The journal is keyed by the local day the editor observed, which is what
    // the timestamp's date is here too.
    let today = clock::utc_timestamp(clock::now())[..10].to_owned();
    let reporters = pulse::reporters(&settings.state_dir, &today);
    let announced = presence::read(&settings.state_dir);

    if reporters.is_empty() && announced.is_empty() {
        println!("editors     no plugin has loaded");
        println!("            reload the editor window; a plugin announces itself when it starts");
    }
    for announcement in &announced {
        let reported = reporters
            .iter()
            .find(|reporter| reporter.editor == announcement.editor);
        match reported {
            Some(reporter) => println!(
                "editors     {} {} — {} pulses today, last {} ago",
                announcement.editor,
                announcement.version,
                reporter.pulses,
                ago(clock::now() - reporter.last_seen)
            ),
            // Loaded but with nothing to report. Typing in the editor produces
            // pulses; an agent writing files behind its back does not.
            None => println!(
                "editors     {} {} — loaded {} ago, nothing typed today",
                announcement.editor,
                announcement.version,
                ago(clock::now() - announcement.at)
            ),
        }
    }
    for reporter in &reporters {
        if !announced
            .iter()
            .any(|announcement| announcement.editor == reporter.editor)
        {
            println!(
                "editors     {} — {} pulses today, last {} ago",
                reporter.editor,
                reporter.pulses,
                ago(clock::now() - reporter.last_seen)
            );
        }
    }

    match fs::read_to_string(&settings.snapshot)
        .ok()
        .and_then(|body| parse_activity_snapshot(&body).ok())
    {
        Some(snapshot) => {
            let totals = summarise_activity(&snapshot.days);
            println!(
                "collected   {} days, agent {}, editor {}",
                snapshot.days.len(),
                clock_face(totals.agent.seconds),
                clock_face(totals.editor.seconds)
            );
        }
        None => println!(
            "collected   nothing readable at {}",
            settings.snapshot.display()
        ),
    }

    Ok(())
}

/// How long ago, in the largest unit that still says something. A negative gap
/// means the reporting machine's clock is ahead of this one, which is worth
/// reading as "just now" rather than as a negative duration.
fn ago(seconds: i64) -> String {
    match seconds.max(0) {
        0..=90 => format!("{}s", seconds.max(0)),
        91..=5_400 => format!("{}m", seconds / 60),
        _ => format!("{}h", seconds / 3_600),
    }
}

fn clock_face(seconds: u64) -> String {
    format!("{}h {}m", seconds / 3_600, (seconds % 3_600) / 60)
}

/// Writes a service description that starts `serve` with the same options this
/// install was given, then asks the system to run it. Paths are made absolute
/// first: a launch agent starts in no particular directory, so a relative
/// snapshot path would land somewhere nobody chose.
fn install(
    args: &[String],
    settings: &Settings,
    prefs: &Preferences,
) -> Result<(), Box<dyn Error>> {
    // Fail before writing anything if the options do not describe a working
    // daemon, so an install never leaves a service that cannot start.
    let _ = sink_for(args, settings, prefs)?;

    let service = service::describe()?;
    let program = env::current_exe()?;
    let logs = settings.state_dir.join("daemon.log");
    let forwarded = forwarded(args, settings)?;

    fs::create_dir_all(&settings.state_dir)?;
    service::write(&service, &service::contents(&program, &forwarded, &logs))?;

    let loader = service::loader();
    let mut command = process::Command::new(loader);
    command.args(&service.load);
    if !cfg!(target_os = "macos") {
        command.arg(format!("{}.service", service::LABEL));
    }
    let outcome = command.output()?;
    if !outcome.status.success() {
        return Err(format!(
            "wrote {} but {loader} refused it: {}",
            service.path.display(),
            String::from_utf8_lossy(&outcome.stderr).trim()
        )
        .into());
    }

    println!("installed {}", service.path.display());
    println!("logs {}", logs.display());
    println!("running as {} and starting at login", service::LABEL);
    Ok(())
}

fn uninstall() -> Result<(), Box<dyn Error>> {
    let service = service::describe()?;
    let loader = service::loader();
    let mut command = process::Command::new(loader);
    command.args(&service.unload);
    if !cfg!(target_os = "macos") {
        command.arg(format!("{}.service", service::LABEL));
    }
    // A service that was never loaded is not an error worth stopping for; the
    // file going away is what was asked for.
    let _ = command.output();
    service::remove(&service)?;

    println!("removed {}", service.path.display());
    Ok(())
}

/// The options to repeat in the service description, with paths resolved so they
/// mean the same thing from a different working directory.
fn forwarded(args: &[String], settings: &Settings) -> Result<Vec<String>, Box<dyn Error>> {
    let mut forwarded = vec![
        "--state".to_owned(),
        settings.state_dir.display().to_string(),
        "--output".to_owned(),
        absolute(&settings.snapshot)?.display().to_string(),
        "--home".to_owned(),
        settings.home.display().to_string(),
        "--idle-timeout".to_owned(),
        (settings.idle_timeout_seconds / 60).to_string(),
    ];

    for name in ["--addr", "--interval", "--sink", "--branch", "--origin"] {
        if let Some(value) = option(args, name) {
            forwarded.push(name.to_owned());
            forwarded.push(value);
        }
    }
    if let Some(repo) = option(args, "--repo") {
        forwarded.push("--repo".to_owned());
        forwarded.push(absolute(&PathBuf::from(repo))?.display().to_string());
    }
    if args.iter().any(|arg| arg == "--no-push") {
        forwarded.push("--no-push".to_owned());
    }

    Ok(forwarded)
}

fn absolute(path: &PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    if path.is_absolute() {
        return Ok(path.clone());
    }
    Ok(env::current_dir()?.join(path))
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
    prefs: &Preferences,
    name: &str,
    fallback: i64,
    least: i64,
    most: i64,
) -> Result<i64, Box<dyn Error>> {
    let value = match chosen(args, prefs, name) {
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
    serve      Listen for pulses and rebuild the snapshot on a timer. The default.
    status     Say whether the daemon is up, which editors have reported today,
               and what has been collected. Use this to tell a plugin that is
               working from one that is merely installed.
    install    Keep serve running at login, with the options given here.
    uninstall  Stop that and remove the service description.
    token      Print where the shared secret lives, and what it is, for a plugin
               that cannot read the file itself.

OPTIONS
    --addr <host:port>    Where to listen. Loopback only. Default {DEFAULT_ADDRESS}.
    --interval <minutes>  How often to rebuild the snapshot. Default {DEFAULT_INTERVAL_MINUTES}.
    --sink <file|git>     Where a rebuilt snapshot goes. Default file.
    --repo <path>         Where to keep the storage checkout, for the git sink.
    --origin <url>        Git remote to clone the checkout from, and push to.
    --branch <name>       Branch to push to. Default master.
    --no-push             Commit locally without pushing.

    Any of these can be written once in <state>/config as `name = value`,
    without the dashes, instead of repeated. A flag overrides the file.
    --idle-timeout <min>  A gap longer than this ends a session rather than
                          counting as time worked. Default {DEFAULT_IDLE_TIMEOUT_MINUTES}.
    --output <path>       Where to write the snapshot. Default <state>/{DEFAULT_SNAPSHOT}.
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
