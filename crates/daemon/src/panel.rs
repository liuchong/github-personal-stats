//! The local panel: one page showing what has been collected on this machine.
//!
//! The numbers are rendered into the page rather than fetched from it, so the
//! browser never needs to hold the token and there is no second endpoint to keep
//! in step with the first.

use github_personal_stats_core::{
    ActivitySnapshot, ActivityTotals, CodingActivityEntry, MEASURE_AGENT, MEASURE_EDITOR,
};

use crate::http::quote;

pub fn page(snapshot: &ActivitySnapshot, totals: &ActivityTotals) -> String {
    let span = match (&totals.first_day, &totals.last_day) {
        (Some(first), Some(last)) => format!("{first} to {last}"),
        _ => "nothing collected yet".to_owned(),
    };
    // The two language columns sit side by side, so they share one scale. Scaling
    // each to its own largest entry would draw two minutes of editor time as long
    // a bar as forty hours of agent time.
    let widest = totals
        .measure(MEASURE_AGENT)
        .languages
        .first()
        .map_or(0, |entry| entry.seconds)
        .max(
            totals
                .measure(MEASURE_EDITOR)
                .languages
                .first()
                .map_or(0, |entry| entry.seconds),
        )
        .max(1);

    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>Activity on {machine}</title>\n<style>{style}</style>\n</head>\n<body>\n\
<header><h1>Activity on {machine}</h1><p>{span} · {days} days recorded · collected {collected}</p></header>\n\
<section class=\"measures\">{measures}</section>\n\
<section class=\"columns\">\n<div><h2>Languages, by agent time</h2>{agent_languages}</div>\n\
<div><h2>Languages, by editor time</h2>{editor_languages}</div>\n</section>\n\
<section class=\"columns\">\n<div><h2>Lines committed</h2>{committed}</div>\n\
<div><h2>Lines generated in the editor</h2>{generated}</div>\n</section>\n\
<section><h2>Models</h2>{models}</section>\n\
<footer><p>Editor time comes from editor plugins reporting what is being worked \
on. Agent time comes from the editor's own record of code it generated. They \
measure different things and a day can be long in one and short in the other.</p></footer>\n\
</body>\n</html>\n",
        machine = escape(&snapshot.machine),
        span = escape(&span),
        days = snapshot.days.len(),
        collected = escape(&snapshot.collected_at),
        style = STYLE,
        measures = measures(totals),
        agent_languages = languages(&totals.measure(MEASURE_AGENT).languages, widest),
        editor_languages = languages(&totals.measure(MEASURE_EDITOR).languages, widest),
        committed = committed(totals),
        generated = generated(totals),
        models = models(totals),
    )
}

pub fn summary_json(snapshot: &ActivitySnapshot, totals: &ActivityTotals) -> String {
    let language_list = |entries: &[CodingActivityEntry]| {
        entries
            .iter()
            .map(|entry| {
                format!(
                    "{{\"language\":{},\"seconds\":{}}}",
                    quote(&entry.language),
                    entry.seconds
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    };

    format!(
        "{{\"machine\":{},\"collectedAt\":{},\"days\":{},\"firstDay\":{},\"lastDay\":{},\
\"activeDays\":{},\"requests\":{},\
\"editor\":{{\"seconds\":{},\"sessions\":{},\"languages\":[{}]}},\
\"agent\":{{\"seconds\":{},\"sessions\":{},\"languages\":[{}]}},\
\"committed\":{{\"added\":{},\"attributed\":{},\"aiShareBasisPoints\":{}}},\
\"generated\":{{\"total\":{},\"aiShareBasisPoints\":{}}},\
\"models\":[{}]}}\n",
        quote(&snapshot.machine),
        quote(&snapshot.collected_at),
        snapshot.days.len(),
        totals.first_day.as_deref().map_or("null".to_owned(), quote),
        totals.last_day.as_deref().map_or("null".to_owned(), quote),
        totals.active_days,
        totals.requests,
        totals.measure(MEASURE_EDITOR).seconds,
        totals.measure(MEASURE_EDITOR).sessions,
        language_list(&totals.measure(MEASURE_EDITOR).languages),
        totals.measure(MEASURE_AGENT).seconds,
        totals.measure(MEASURE_AGENT).sessions,
        language_list(&totals.measure(MEASURE_AGENT).languages),
        totals.commits.added(),
        totals.commits.attributed_added(),
        totals.commits.ai_share_basis_points(),
        totals.lines.total(),
        totals.lines.ai_share_basis_points(),
        totals
            .models()
            .iter()
            .map(|model| format!(
                "{{\"name\":{},\"lines\":{}}}",
                quote(&model.name),
                model.lines
            ))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn measures(totals: &ActivityTotals) -> String {
    format!(
        "{}{}{}",
        card(
            "Editor time",
            &clock(totals.measure(MEASURE_EDITOR).seconds),
            &format!("{} sessions", totals.measure(MEASURE_EDITOR).sessions),
        ),
        card(
            "Agent time",
            &clock(totals.measure(MEASURE_AGENT).seconds),
            &format!("{} sessions", totals.measure(MEASURE_AGENT).sessions),
        ),
        card(
            "Requests",
            &totals.requests.to_string(),
            &format!("{} active days", totals.active_days),
        ),
    )
}

fn card(label: &str, value: &str, note: &str) -> String {
    format!(
        "<div class=\"card\"><p class=\"label\">{}</p><p class=\"value\">{}</p><p class=\"note\">{}</p></div>",
        escape(label),
        escape(value),
        escape(note)
    )
}

fn languages(entries: &[CodingActivityEntry], largest: u64) -> String {
    if entries.is_empty() {
        return "<p class=\"empty\">Nothing recorded.</p>".to_owned();
    }
    let rows = entries
        .iter()
        .take(8)
        .map(|entry| {
            format!(
                "<tr><td>{}</td><td class=\"bar\"><span style=\"width:{}%\"></span></td><td class=\"figure\">{}</td></tr>",
                escape(&entry.language),
                share(entry.seconds, largest),
                escape(&clock(entry.seconds))
            )
        })
        .collect::<String>();
    format!("<table>{rows}</table>")
}

fn committed(totals: &ActivityTotals) -> String {
    let counts = &totals.commits;
    rows(&[
        ("Added", counts.added().to_string()),
        ("Attributable", counts.attributed_added().to_string()),
        (
            "AI share of attributable",
            percentage(counts.ai_share_basis_points()),
        ),
        (
            "Attributable share of all",
            percentage(counts.attributed_share_basis_points()),
        ),
    ])
}

fn generated(totals: &ActivityTotals) -> String {
    let lines = &totals.lines;
    rows(&[
        ("Lines", lines.total().to_string()),
        ("By an agent", lines.authors.agent.total().to_string()),
        ("Not by an agent", lines.authors.human.total().to_string()),
        ("AI share", percentage(lines.ai_share_basis_points())),
    ])
}

fn models(totals: &ActivityTotals) -> String {
    if totals.models().is_empty() {
        return "<p class=\"empty\">Nothing recorded.</p>".to_owned();
    }
    let largest = totals
        .models()
        .first()
        .map_or(1, |model| model.lines)
        .max(1);
    let rows = totals
        .models()
        .iter()
        .take(10)
        .map(|model| {
            format!(
                "<tr><td>{}</td><td class=\"bar\"><span style=\"width:{}%\"></span></td><td class=\"figure\">{} lines</td></tr>",
                escape(&model.name),
                share(model.lines, largest),
                model.lines
            )
        })
        .collect::<String>();
    format!("<table>{rows}</table>")
}

fn rows(pairs: &[(&str, String)]) -> String {
    let body = pairs
        .iter()
        .map(|(label, value)| {
            format!(
                "<tr><td>{}</td><td class=\"figure\">{}</td></tr>",
                escape(label),
                escape(value)
            )
        })
        .collect::<String>();
    format!("<table>{body}</table>")
}

/// A bar wide enough to see. Something small next to something large rounds to
/// nothing, and a missing bar reads as no time at all rather than a little.
fn share(value: u64, largest: u64) -> u64 {
    match value {
        0 => 0,
        _ => (value * 100 / largest.max(1)).max(1),
    }
}

fn percentage(basis_points: u32) -> String {
    format!("{}.{}%", basis_points / 100, (basis_points % 100) / 10)
}

fn clock(seconds: u64) -> String {
    format!("{} hrs {} mins", seconds / 3_600, (seconds % 3_600) / 60)
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const STYLE: &str = "\
:root{--ink:#1f2328;--soft:#59636e;--line:#d1d9e0;--paper:#ffffff;--fill:#f6f8fa;--accent:#0969da}\
@media(prefers-color-scheme:dark){:root{--ink:#f0f6fc;--soft:#9198a1;--line:#3d444d;--paper:#0d1117;--fill:#151b23;--accent:#4493f8}}\
*{box-sizing:border-box}\
body{margin:0;padding:32px;background:var(--paper);color:var(--ink);\
font:15px/1.5 -apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif;\
font-variant-numeric:tabular-nums;max-width:1000px;margin-inline:auto}\
h1{font-size:20px;margin:0 0 4px}h2{font-size:13px;text-transform:uppercase;letter-spacing:.06em;\
color:var(--soft);margin:0 0 12px;font-weight:600}\
header p{margin:0;color:var(--soft);font-size:13px}\
section{margin-top:32px}\
.measures{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:16px}\
.card{border:1px solid var(--line);border-radius:8px;padding:16px;background:var(--fill)}\
.card .label{margin:0;font-size:12px;color:var(--soft);text-transform:uppercase;letter-spacing:.06em}\
.card .value{margin:6px 0 0;font-size:24px;font-weight:600}\
.card .note{margin:2px 0 0;font-size:12px;color:var(--soft)}\
.columns{display:grid;grid-template-columns:repeat(auto-fit,minmax(320px,1fr));gap:32px}\
table{width:100%;border-collapse:collapse}\
td{padding:5px 0;border-bottom:1px solid var(--line);font-size:14px}\
td.figure{text-align:right;color:var(--soft);white-space:nowrap}\
td.bar{width:50%;padding-inline:12px}\
td.bar span{display:block;height:6px;border-radius:3px;background:var(--accent)}\
.empty{color:var(--soft);font-size:14px;margin:0}\
footer{margin-top:40px;border-top:1px solid var(--line);padding-top:16px}\
footer p{color:var(--soft);font-size:13px;margin:0}";
