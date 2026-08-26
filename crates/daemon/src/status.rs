//! What to say when asked whether any of this is working.
//!
//! Each line names one thing that can be separately broken, because the useful
//! answer is not "yes" or "no" but which part is not doing its job. The reading
//! lives here rather than in the binary so it can be exercised against states
//! that are awkward to arrange by hand: a plugin loaded but idle, a plugin
//! reporting from a version nobody expected, an editor heard from that never
//! announced itself.

use github_personal_stats_collect::{presence::Announcement, pulse::Reporter};

/// Everything the report is drawn from, gathered by the caller so that reading it
/// stays a pure function of what was found.
pub struct Reading<'a> {
    pub address: &'a str,
    pub listening: bool,
    pub token: Option<&'a str>,
    pub publishing: &'a str,
    pub announced: &'a [Announcement],
    pub reporters: &'a [Reporter],
    /// Now, for measuring how long ago things happened.
    pub at: i64,
    pub collected: Option<Collected>,
}

pub struct Collected {
    pub days: usize,
    pub agent_seconds: u64,
    pub editor_seconds: u64,
}

pub fn report(reading: &Reading) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "daemon      {}",
        if reading.listening {
            format!("listening on {}", reading.address)
        } else {
            format!("not listening on {}", reading.address)
        }
    ));

    lines.push(match reading.token {
        Some(path) => format!("token       {path}"),
        None => "token       missing — no plugin can report".to_owned(),
    });

    lines.push(format!("publishing  {}", reading.publishing));
    lines.extend(editors(reading));

    lines.push(match &reading.collected {
        Some(collected) => format!(
            "collected   {} days, agent {}, editor {}",
            collected.days,
            clock_face(collected.agent_seconds),
            clock_face(collected.editor_seconds)
        ),
        None => "collected   nothing readable yet".to_owned(),
    });

    lines.join("\n")
}

/// The editor lines. A plugin announces itself when it loads and reports only
/// when there is work, so the two facts are separate and both worth saying: a
/// loaded plugin in a window nobody is typing in is working correctly, and looks
/// like nothing at all if only pulses are counted.
fn editors(reading: &Reading) -> Vec<String> {
    let mut lines = Vec::new();

    if reading.announced.is_empty() && reading.reporters.is_empty() {
        lines.push("editors     no plugin has loaded".to_owned());
        lines.push(
            "            reload the editor window; a plugin announces itself when it starts"
                .to_owned(),
        );
        return lines;
    }

    for announcement in reading.announced {
        let named = named(&announcement.editor, &announcement.version);
        match reading
            .reporters
            .iter()
            .find(|reporter| reporter.editor == announcement.editor)
        {
            Some(reporter) => lines.push(format!(
                "editors     {named} — {} on {}, last {} ago",
                pulses(reporter.pulses),
                reporter.day,
                ago(reading.at - reporter.last_seen)
            )),
            // A window being worked in produces pulses; one sitting in the
            // background does not. Saying so keeps a correct state from reading
            // as a fault.
            None => lines.push(format!(
                "editors     {named} — loaded {} ago, nothing reported recently",
                ago(reading.at - announcement.at)
            )),
        }
    }

    // An editor heard from without an announcement is an older plugin, or one
    // whose hello was lost. Its work still counts.
    for reporter in reading.reporters {
        if !reading
            .announced
            .iter()
            .any(|announcement| announcement.editor == reporter.editor)
        {
            lines.push(format!(
                "editors     {} — {} on {}, last {} ago",
                reporter.editor,
                pulses(reporter.pulses),
                reporter.day,
                ago(reading.at - reporter.last_seen)
            ));
        }
    }

    lines
}

fn pulses(count: usize) -> String {
    if count == 1 {
        "1 pulse".to_owned()
    } else {
        format!("{count} pulses")
    }
}

fn named(editor: &str, version: &str) -> String {
    if version.is_empty() {
        editor.to_owned()
    } else {
        format!("{editor} {version}")
    }
}

/// How long ago, in the largest unit that still says something. A negative gap
/// means the reporting machine's clock is ahead of this one, which is worth
/// reading as "just now" rather than as a negative duration.
pub fn ago(seconds: i64) -> String {
    let seconds = seconds.max(0);
    match seconds {
        0..=90 => format!("{seconds}s"),
        91..=5_400 => format!("{}m", seconds / 60),
        _ => format!("{}h", seconds / 3_600),
    }
}

pub fn clock_face(seconds: u64) -> String {
    format!("{}h {}m", seconds / 3_600, (seconds % 3_600) / 60)
}
