//! Choosing what a chart says, as opposed to how it looks.
//!
//! Collection records facts. `textchart` lays out rows. This is the joint between
//! them: it reads a spec — a value and a dimension, `lines/models`, `time/languages` —
//! and folds the record into rows for it.
//!
//! Keeping the joint in one small place is what makes the two sides independent.
//! A new dimension is a new arm of one match here, needing nothing from the
//! collector and nothing from the layout. A new look is a `ChartStyle`, needing
//! nothing from either.

use std::collections::BTreeMap;

use crate::{
    ActivityComparison, ActivityWindow, Author, AuthorShare, GithubStatsError, LineFact,
    language_label,
    renderer::{format_duration_aligned, format_number as thousands},
    textchart::{ChartBlock, ChartRow, ChartSummary},
};

/// What a block counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartValue {
    /// Seconds of the measure the comparison was built for.
    Time,
    /// Lines written.
    Lines,
    /// Tokens spent, for the sources that report them.
    Tokens,
}

/// What a block's rows are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartRows {
    Languages,
    Models,
    Authors,
    /// One row per window, for a block that compares the spans against each other
    /// rather than breaking one of them down.
    Windows,
}

/// One block of a chart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSpec {
    pub value: ChartValue,
    pub rows: ChartRows,
    /// How many rows at most. The rest are dropped rather than gathered into an
    /// "other" row, because a row called other in a block of the largest few is
    /// not the same quantity as the remainder and saying so takes more room than
    /// it is worth.
    pub limit: usize,
    /// Whether to divide each bar between the agent and the person. Ignored where
    /// the value cannot be divided.
    pub split: bool,
    /// Heading, or empty to use the one that fits the spec.
    pub title: String,
    /// Which measure of time this block reads, or empty for the chart's own.
    ///
    /// A measure belongs to the block rather than to the chart because the
    /// interesting charts hold more than one: hours an agent spent changing code
    /// and hours carried in from elsewhere are different quantities that overlap,
    /// so they can be shown side by side but never added, and a chart with a
    /// single measure could only show one of them at a time.
    pub measure: String,
    /// Whether each row should say who wrote its lines.
    ///
    /// A block of hours cannot answer that from its own figures: the hours it
    /// divides are hours an agent was seen working, and on a record where an
    /// agent does nearly all of it the division is a rounding error and draws as
    /// nothing. Lines do answer it, and they answer it per language, so a block
    /// of hours can carry that answer beside each language as long as it says
    /// that is what it is. The two are never added or drawn in the same bar,
    /// since a bar whose length is time and whose parts are lines would invite
    /// exactly the wrong reading.
    pub authors: bool,
}

impl BlockSpec {
    pub fn new(value: ChartValue, rows: ChartRows) -> Self {
        Self {
            value,
            rows,
            limit: 6,
            split: true,
            authors: false,
            title: String::new(),
            measure: String::new(),
        }
    }

    /// The same block, read from a named measure.
    pub fn of(mut self, measure: &str) -> Self {
        self.measure = measure.to_owned();
        self
    }

    /// Reads one block from `value/rows` with optional `,key=value` settings.
    ///
    /// For example `lines/languages,limit=8,split=off,title=By language`.
    pub fn parse(spec: &str) -> Result<Self, GithubStatsError> {
        let mut parts = spec.split(',');
        let head = parts.next().unwrap_or_default().trim();
        let (value, rows) = head.split_once('/').ok_or_else(|| {
            invalid(&format!(
                "block {head:?} must start with value/rows, such as time/languages or lines/models"
            ))
        })?;

        let value = match value.trim() {
            "time" => ChartValue::Time,
            "lines" => ChartValue::Lines,
            "tokens" => ChartValue::Tokens,
            other => {
                return Err(invalid(&format!(
                    "unknown value {other:?}; expected time, lines, or tokens"
                )));
            }
        };
        let rows = match rows.trim() {
            "languages" => ChartRows::Languages,
            "models" => ChartRows::Models,
            "authors" => ChartRows::Authors,
            "windows" => ChartRows::Windows,
            other => {
                return Err(invalid(&format!(
                    "unknown dimension {other:?}; expected languages, models, authors, or windows"
                )));
            }
        };

        let mut block = Self::new(value, rows);
        for setting in parts {
            let setting = setting.trim();
            if setting.is_empty() {
                continue;
            }
            let (key, held) = setting
                .split_once('=')
                .ok_or_else(|| invalid(&format!("block setting {setting:?} must be key=value")))?;
            match key.trim() {
                "limit" => {
                    block.limit = held.trim().parse().map_err(|_| {
                        invalid(&format!("limit {:?} must be a whole number", held.trim()))
                    })?;
                }
                "split" => block.split = matches!(held.trim(), "on" | "true" | "yes" | "author"),
                "authors" => block.authors = matches!(held.trim(), "on" | "true" | "yes"),
                "title" => block.title = held.trim().to_owned(),
                "measure" => block.measure = held.trim().to_owned(),
                other => {
                    return Err(invalid(&format!(
                        "unknown block setting {other:?}; \
                         expected limit, split, authors, title, or measure"
                    )));
                }
            }
        }
        Ok(block)
    }

    fn heading(&self) -> String {
        if !self.title.is_empty() {
            return self.title.clone();
        }
        // A block of time says which time it means when it was asked for by name,
        // because a chart holding two measures needs to tell them apart, and one
        // holding a single unnamed measure has nothing to tell apart.
        let value = match self.value {
            ChartValue::Time if !self.measure.is_empty() => {
                return format!("{}  {}", self.time_heading(), self.row_heading());
            }
            ChartValue::Time => "TIME",
            ChartValue::Lines => "LINES",
            ChartValue::Tokens => "TOKENS",
        };
        format!("{value}  {}", self.row_heading())
    }

    fn time_heading(&self) -> String {
        format!("{} TIME", self.measure.to_uppercase())
    }

    fn row_heading(&self) -> &'static str {
        match self.rows {
            ChartRows::Languages => "BY LANGUAGE",
            ChartRows::Models => "BY MODEL",
            ChartRows::Authors => "BY AUTHOR",
            ChartRows::Windows => "BY SPAN",
        }
    }
}

/// Reads a whole chart: blocks separated by semicolons.
pub fn parse_blocks(spec: &str) -> Result<Vec<BlockSpec>, GithubStatsError> {
    spec.split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(BlockSpec::parse)
        .collect()
}

/// What a chart shows when nothing was asked for: how long, who wrote it, and
/// with what. Three questions that between them say what a day of this work was.
pub fn default_blocks() -> Vec<BlockSpec> {
    vec![
        BlockSpec::new(ChartValue::Time, ChartRows::Languages),
        BlockSpec::new(ChartValue::Lines, ChartRows::Authors),
        BlockSpec::new(ChartValue::Lines, ChartRows::Models),
    ]
}

/// Folds comparisons into blocks, each block reading the measure it named.
///
/// The first fold is what a block that named no measure reads. A block naming a
/// measure that was never folded draws as empty, which is what a measure with
/// nothing behind it means.
pub fn build_blocks(folds: &[ActivityComparison], specs: &[BlockSpec]) -> Vec<ChartBlock> {
    specs
        .iter()
        .map(|spec| match pick(folds, &spec.measure) {
            Some(comparison) => build_block(comparison, spec),
            None => ChartBlock::new(spec.heading()),
        })
        .collect()
}

fn pick<'a>(folds: &'a [ActivityComparison], measure: &str) -> Option<&'a ActivityComparison> {
    if measure.is_empty() {
        return folds.first();
    }
    folds.iter().find(|fold| fold.measure.as_str() == measure)
}

fn build_block(comparison: &ActivityComparison, spec: &BlockSpec) -> ChartBlock {
    let window = &comparison.recent;

    let (rows, total) = match (spec.value, spec.rows) {
        (ChartValue::Time, ChartRows::Languages) => language_time(comparison, spec),
        (ChartValue::Time, ChartRows::Windows) => window_time(comparison),
        (ChartValue::Lines, ChartRows::Languages) => language_lines(comparison, spec),
        (ChartValue::Lines, ChartRows::Authors) => author_lines(window),
        (ChartValue::Lines, ChartRows::Models) => model_lines(window, spec),
        (ChartValue::Lines, ChartRows::Windows) => window_lines(comparison),
        (ChartValue::Tokens, ChartRows::Models) => model_tokens(window, spec),
        // Time has no author to divide by, and tokens are not recorded against a
        // language or an author. Rather than invent a number, the block says so.
        _ => (Vec::new(), 0),
    };

    let block = ChartBlock::new(spec.heading());
    let block = match summary(comparison, spec, total) {
        Some(summary) => block.with_summary(summary),
        None => block,
    };
    // A bar is only ever divided by authorship at present, so that is what its
    // parts are called. The words live here rather than in the layout because the
    // layout has no way of knowing what a bar was divided by.
    block.with_rows(rows, total).divided_into(AGENT, NOT_AGENT)
}

/// What the record can say about who wrote a line.
///
/// It knows when a change came from an agent it was watching. Everything else is
/// only known not to have, which is a weaker claim than a person having typed it:
/// a checkout, a rebase, a formatter or a scripted rewrite all land in it. Saying
/// "me" would claim the stronger thing, and only an editor plugin watching
/// keystrokes can honestly claim that.
const AGENT: &str = "agent";
const NOT_AGENT: &str = "not by an agent";

/// The block's total.
///
/// It is the total of what the block actually shows rather than a figure taken
/// from the window, because those are not always the same quantity: a block of
/// lines by model totals what the models wrote, which is not what everyone wrote.
/// Putting the window's figure there would make the percentages look wrong when
/// they are right.
fn summary(comparison: &ActivityComparison, spec: &BlockSpec, total: u64) -> Option<ChartSummary> {
    if total == 0 {
        return None;
    }
    // Spans contain one another — thirty days is part of all time — so a block of
    // them has no total, and its percentages are each span against the longest.
    // Saying so is the difference between a reader trusting the figures and
    // trying to add them up.
    if spec.rows == ChartRows::Windows {
        return Some(ChartSummary::new(
            "Longest",
            match spec.value {
                ChartValue::Time => format_duration_aligned(total),
                _ => thousands(total),
            },
            "spans overlap; each reads as a share of this",
        ));
    }

    let window = &comparison.recent;
    Some(match spec.value {
        ChartValue::Time => ChartSummary::new(
            "Total",
            format_duration_aligned(total),
            format!(
                "{}, {}",
                window.span.label().to_lowercase(),
                window.coverage()
            ),
        ),
        ChartValue::Lines => {
            let note = match spec.rows {
                // A block already broken down by author would only be repeating
                // itself if the total named the split too, so it says the span
                // instead: without it the figure is a count of lines over an
                // unstated period.
                ChartRows::Authors => format!("lines, {}", window.span.label().to_lowercase()),
                _ => format!(
                    "lines, {} by an agent",
                    percent(window.lines.ai_share_basis_points())
                ),
            };
            ChartSummary::new("Total", thousands(total), note)
        }
        ChartValue::Tokens => ChartSummary::new("Total", thousands(total), "tokens billed"),
    })
}

fn language_time(comparison: &ActivityComparison, spec: &BlockSpec) -> (Vec<ChartRow>, u64) {
    let total = comparison
        .languages
        .iter()
        .map(|language| language.recent_seconds)
        .sum();
    let rows = comparison
        .languages
        .iter()
        .filter(|language| language.recent_seconds > 0)
        .take(spec.limit)
        .map(|language| {
            let row = ChartRow::new(
                language_label(&language.name),
                format_duration_aligned(language.recent_seconds),
                language.recent_seconds,
            );
            // A source that could not say who spent the time leaves both parts
            // at zero, and the bar is drawn whole rather than pretending the
            // whole of it was unattributed.
            let row = if spec.split {
                row.divided(language.attributed.agent, language.attributed.human)
            } else {
                row
            };
            if spec.authors {
                row.with_aside(authorship(&language.lines))
            } else {
                row
            }
        })
        .collect();
    (rows, total)
}

/// How a language's lines divide, for a row whose figure is not lines.
///
/// It names what it counted. On a block of hours the reader has every reason to
/// assume a percentage refers to the hours, and this one does not.
fn authorship(lines: &AuthorShare) -> String {
    if lines.total() == 0 {
        return String::new();
    }
    format!("{} agent lines", percent(lines.ai_share_basis_points()))
}

fn language_lines(comparison: &ActivityComparison, spec: &BlockSpec) -> (Vec<ChartRow>, u64) {
    let mut ranked = comparison
        .languages
        .iter()
        .filter(|language| language.lines.total() > 0)
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .lines
            .total()
            .cmp(&left.lines.total())
            .then_with(|| left.name.cmp(&right.name))
    });
    let total = ranked.iter().map(|language| language.lines.total()).sum();
    let rows = ranked
        .into_iter()
        .take(spec.limit)
        .map(|language| {
            let row = ChartRow::new(
                language_label(&language.name),
                thousands(language.lines.total()),
                language.lines.total(),
            );
            if spec.split {
                row.divided(language.lines.agent, language.lines.human)
            } else {
                row
            }
        })
        .collect();
    (rows, total)
}

fn author_lines(window: &ActivityWindow) -> (Vec<ChartRow>, u64) {
    let total = window.lines.total();
    let rows = [(AGENT, window.lines.agent), (NOT_AGENT, window.lines.human)]
        .into_iter()
        .filter(|(_, lines)| *lines > 0 || total > 0)
        .map(|(name, lines)| ChartRow::new(name, thousands(lines), lines))
        .collect();
    (rows, total)
}

fn model_lines(window: &ActivityWindow, spec: &BlockSpec) -> (Vec<ChartRow>, u64) {
    let models = window.models();
    let total = models.iter().map(|(_, lines)| lines).sum();
    let rows = models
        .into_iter()
        .take(spec.limit)
        .map(|(name, lines)| ChartRow::new(name, thousands(lines), lines))
        .collect();
    (rows, total)
}

fn model_tokens(window: &ActivityWindow, spec: &BlockSpec) -> (Vec<ChartRow>, u64) {
    let spend = window.token_spend();
    let total = spend.iter().map(|(_, billed)| billed).sum();
    let rows = spend
        .into_iter()
        .take(spec.limit)
        .map(|(name, billed)| ChartRow::new(name, thousands(billed), billed))
        .collect();
    (rows, total)
}

fn window_time(comparison: &ActivityComparison) -> (Vec<ChartRow>, u64) {
    let windows = [&comparison.recent, &comparison.baseline];
    let total = windows
        .iter()
        .map(|window| window.seconds)
        .max()
        .unwrap_or(0);
    let rows = windows
        .into_iter()
        .map(|window| {
            ChartRow::new(
                window.span.label(),
                format_duration_aligned(window.seconds),
                window.seconds,
            )
        })
        .collect();
    (rows, total)
}

fn window_lines(comparison: &ActivityComparison) -> (Vec<ChartRow>, u64) {
    let windows = [&comparison.recent, &comparison.baseline];
    let total = windows
        .iter()
        .map(|window| window.lines.total())
        .max()
        .unwrap_or(0);
    let rows = windows
        .into_iter()
        .map(|window| {
            ChartRow::new(
                window.span.label(),
                thousands(window.lines.total()),
                window.lines.total(),
            )
            .divided(window.lines.agent, window.lines.human)
        })
        .collect();
    (rows, total)
}

/// Sums facts by language, for callers folding a record directly rather than
/// through a comparison.
pub fn fold_by_language(facts: &[LineFact]) -> BTreeMap<String, (u64, u64)> {
    let mut held = BTreeMap::<String, (u64, u64)>::new();
    for fact in facts {
        let entry = held.entry(fact.language.clone()).or_default();
        match fact.author {
            Author::Agent => entry.0 += fact.total(),
            Author::Human => entry.1 += fact.total(),
        }
    }
    held
}

fn percent(basis_points: u32) -> String {
    format!("{}.{:02}%", basis_points / 100, basis_points % 100)
}

fn invalid(message: &str) -> GithubStatsError {
    GithubStatsError::InvalidConfig {
        field: "activity-blocks",
        message: message.to_owned(),
    }
}
