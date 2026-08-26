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
    ActivityComparison, ActivityWindow, Author, GithubStatsError, LineFact, LineShare, Lines,
    language_label,
    renderer::{format_duration_aligned, format_number as thousands},
    textchart::{ChartBlock, ChartRow, ChartSummary},
};

/// What a block counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartValue {
    /// Seconds of the measure the comparison was built for.
    Time,
    /// Lines the editor watched being written.
    ///
    /// Additions only, and no removal will ever appear beside them. The editor
    /// keeps a row per line that exists, so a line that was deleted stops having
    /// one and there is nothing left to count. Reporting a removal needs each
    /// edit watched as it happens, which is a plugin's job and not a record's.
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
    /// Whether each row should say how long it took.
    ///
    /// Off by default, and a setting rather than a second block, because time is
    /// the weaker of the two figures: a count of lines is a count, while hours
    /// are inferred from the gaps between moments something was seen happening
    /// and are only as good as how densely those moments were observed. A reader
    /// who wants them can have them against any breakdown — time is recorded at
    /// the same grain as lines — but they are not what a chart leads with.
    pub time: bool,
}

impl BlockSpec {
    pub fn new(value: ChartValue, rows: ChartRows) -> Self {
        Self {
            value,
            rows,
            limit: 6,
            split: true,
            authors: false,
            time: false,
            title: String::new(),
            measure: String::new(),
        }
    }

    /// The same block, with each row saying how long it took.
    pub fn timed(mut self) -> Self {
        self.time = true;
        self
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
                "time" => block.time = matches!(held.trim(), "on" | "true" | "yes"),
                "title" => block.title = held.trim().to_owned(),
                "measure" => block.measure = held.trim().to_owned(),
                other => {
                    return Err(invalid(&format!(
                        "unknown block setting {other:?}; \
                         expected limit, split, authors, time, title, or measure"
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
                return format!("{} {}", self.time_heading(), self.row_heading());
            }
            ChartValue::Time => "TIME",
            ChartValue::Lines => "LINES",
            ChartValue::Tokens => "TOKENS",
        };
        format!("{value} {}", self.row_heading())
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

/// What a chart shows when nothing was asked for: what was written, who wrote it,
/// and with what. Three questions that between them say what this work was.
///
/// Lines rather than hours, because a line is counted and an hour is inferred.
/// Every one of these blocks will also state the hours behind its figures on
/// request, which is the right way round: the reader who wants the weaker measure
/// asks for it, rather than the reader who wants the stronger one having to know
/// to turn the other off.
pub fn default_blocks() -> Vec<BlockSpec> {
    vec![
        BlockSpec::new(ChartValue::Lines, ChartRows::Languages),
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

/// What a fold produced: rows, and the block's total behind them.
///
/// The total is carried twice because it answers two questions in two shapes. As
/// a number it is what each row's percentage is a percentage of. As a change it
/// is the figure printed above the rows, and there it has to say additions and
/// removals apart for the same reason the rows do.
struct Fold {
    rows: Vec<ChartRow>,
    total: u64,
    written: Lines,
}

impl Fold {
    /// A fold of something that is not a count of lines, where the total is just
    /// a number and whoever words it knows what unit it is in.
    fn plain(rows: Vec<ChartRow>, total: u64) -> Self {
        Self {
            rows,
            total,
            written: Lines::default(),
        }
    }

    fn counted(rows: Vec<ChartRow>, written: Lines) -> Self {
        Self {
            rows,
            total: written.total(),
            written,
        }
    }
}

fn build_block(comparison: &ActivityComparison, spec: &BlockSpec) -> ChartBlock {
    let window = &comparison.recent;

    let fold = match (spec.value, spec.rows) {
        (ChartValue::Time, ChartRows::Languages) => language_time(comparison, spec),
        (ChartValue::Time, ChartRows::Windows) => window_time(comparison),
        (ChartValue::Lines, ChartRows::Languages) => language_lines(comparison, spec),
        (ChartValue::Lines, ChartRows::Authors) => author_lines(window, spec),
        (ChartValue::Lines, ChartRows::Models) => model_lines(window, spec),
        (ChartValue::Lines, ChartRows::Windows) => window_lines(comparison, spec),
        (ChartValue::Tokens, ChartRows::Models) => model_tokens(window, spec),
        // Tokens are not recorded against a language or an author, and a span is
        // not a thing an author writes. Rather than invent a number for a
        // breakdown nothing measured, the block says it has nothing.
        _ => Fold::plain(Vec::new(), 0),
    };

    let block = ChartBlock::new(spec.heading());
    let block = match summary(comparison, spec, &fold) {
        Some(summary) => block.with_summary(summary),
        None => block,
    };
    let (rows, total) = (fold.rows, fold.total);
    // A bar is only ever divided by authorship at present, so that is what its
    // parts are called. The words live here rather than in the layout because the
    // layout has no way of knowing what a bar was divided by.
    block
        .with_rows(rows, total)
        .divided_into(AGENT, UNATTRIBUTED)
}

/// What the record can say about who wrote a line.
///
/// It knows when a change came from an agent it was watching. Everything else is
/// only known not to have, which is a weaker claim than a person having typed it:
/// a checkout, a rebase, a formatter or a scripted rewrite all land in it. Saying
/// "me" would claim the stronger thing, and only an editor plugin watching
/// keystrokes can honestly claim that.
const AGENT: &str = "agent";
/// Everything no request accounts for, which is not the same as everything a
/// person did. A shell command, a formatter and a terminal agent all land here,
/// so the row says what is true of all of them rather than guessing which.
const UNATTRIBUTED: &str = "unattributed";

/// The block's total.
///
/// It is the total of what the block actually shows rather than a figure taken
/// from the window, because those are not always the same quantity: a block of
/// lines by model totals what the models wrote, which is not what everyone wrote.
/// Putting the window's figure there would make the percentages look wrong when
/// they are right.
fn summary(comparison: &ActivityComparison, spec: &BlockSpec, fold: &Fold) -> Option<ChartSummary> {
    let total = fold.total;
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
                ChartValue::Lines => change(fold.written),
                ChartValue::Tokens => thousands(total),
            },
            "spans overlap; each reads as a share of this",
        ));
    }

    let window = &comparison.recent;
    Some(match spec.value {
        ChartValue::Time => ChartSummary::new(
            "Total",
            format_duration_aligned(total),
            match spec.rows {
                // A block of hours by language totals only the hours it could
                // place, so it has to say what it left out, or its total reads as
                // disagreeing with every other block of the same measure.
                ChartRows::Languages => format!(
                    "{}, {} not placed to a language",
                    window.span.label().to_lowercase(),
                    format_duration_aligned(window.seconds.saturating_sub(total)).trim()
                ),
                _ => format!(
                    "{}, {}",
                    window.span.label().to_lowercase(),
                    window.coverage()
                ),
            },
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
            ChartSummary::new("Total", change(fold.written), note)
        }
        ChartValue::Tokens => ChartSummary::new("Total", thousands(total), "tokens billed"),
    })
}

/// Hours by language, over the hours that have one.
///
/// Most of the measured time has no language and cannot be given one. An hour is
/// counted from moments an agent was seen working, and the terminal agents — which
/// account for more of that than the editor does — say when they were working
/// without saying what on. Measured across this record, one in eight of their
/// moments names a file at all, and a file named at one instant says nothing about
/// the minutes either side of it.
///
/// So the unplaced hours are left out of the rows and out of the total, and the
/// summary says how many they were. Ranking them as a language called unknown
/// would put two thirds of the block in a row that is not a language, and
/// dropping them silently would leave a total that quietly disagrees with every
/// other block. Lines are kept the other way round, as a row: there the unnamed
/// part is a rounding error and it does mean a real thing, a file whose extension
/// names no language.
fn language_time(comparison: &ActivityComparison, spec: &BlockSpec) -> Fold {
    let placed = || comparison.placed();
    let total = placed().map(|language| language.recent_seconds).sum();
    let rows = placed()
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
                row.with_aside(authorship(&language.lines, false))
            } else {
                row
            }
        })
        .collect();
    Fold::plain(rows, total)
}

/// How a row's lines divide, for a reader who wants the number and not the bar.
///
/// Whether it has to name what it counted depends on the row it sits beside. On
/// a block of hours a percentage would be taken for a share of the hours, so
/// there it says which figure it means; on a block of lines it is already
/// talking about the figure to its left, and saying so again would only widen
/// the column.
fn authorship(lines: &LineShare, of_lines: bool) -> String {
    if lines.total() == 0 {
        return String::new();
    }
    let share = percent(lines.ai_share_basis_points());
    if of_lines {
        format!("{share} agent")
    } else {
        format!("{share} agent lines")
    }
}

/// A count of lines, said as a change rather than as a quantity.
///
/// The sign is not decoration: added and removed lines are different events, and
/// a source that only ever sees one of them should not be read as having weighed
/// both. Where nothing was removed the figure says only what was put in, because
/// a written `-0` claims a measurement that was never taken.
fn change(lines: Lines) -> String {
    if lines.deleted == 0 {
        return format!("+{}", thousands(lines.added));
    }
    format!("+{} -{}", thousands(lines.added), thousands(lines.deleted))
}

fn language_lines(comparison: &ActivityComparison, spec: &BlockSpec) -> Fold {
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
    let total = ranked.iter().fold(Lines::default(), |mut held, language| {
        held.absorb(&written(&language.lines));
        held
    });
    let rows = ranked
        .into_iter()
        .take(spec.limit)
        .map(|language| {
            let row = ChartRow::new(
                language_label(&language.name),
                change(written(&language.lines)),
                language.lines.total(),
            );
            let row = if spec.split {
                row.divided(language.lines.agent.total(), language.lines.human.total())
            } else {
                row
            };
            let row = if spec.authors {
                row.with_aside(authorship(&language.lines, true))
            } else {
                row
            };
            timed(row, spec, language.recent_seconds)
        })
        .collect();
    Fold::counted(rows, total)
}

/// The additions and removals of a split, disregarding who made them.
fn written(lines: &LineShare) -> Lines {
    Lines {
        added: lines.added(),
        deleted: lines.deleted(),
    }
}

/// Attaches the hours behind a row's figure, where the block asked for them.
///
/// A row with no recorded time gets an empty remark rather than a zero, which is
/// the difference between "no hours were observed against this" and "this took no
/// time". Where no row in the block has any, the column is not drawn at all.
fn timed(row: ChartRow, spec: &BlockSpec, seconds: u64) -> ChartRow {
    if !spec.time {
        return row;
    }
    if seconds == 0 {
        return row.with_aside(String::new());
    }
    row.with_aside(format_duration_aligned(seconds))
}

fn author_lines(window: &ActivityWindow, spec: &BlockSpec) -> Fold {
    let total = written(&window.lines.authors);
    let rows = [
        (AGENT, window.lines.authors.agent, window.time.authors.agent),
        (
            UNATTRIBUTED,
            window.lines.authors.human,
            window.time.authors.human,
        ),
    ]
    .into_iter()
    .filter(|(_, lines, _)| lines.total() > 0 || total.total() > 0)
    .map(|(name, lines, seconds)| {
        timed(
            ChartRow::new(name, change(lines), lines.total()),
            spec,
            seconds,
        )
    })
    .collect();
    Fold::counted(rows, total)
}

fn model_lines(window: &ActivityWindow, spec: &BlockSpec) -> Fold {
    let models = window.models();
    let total = models
        .iter()
        .fold(Lines::default(), |mut held, (_, lines)| {
            held.absorb(lines);
            held
        });
    let rows = models
        .into_iter()
        .take(spec.limit)
        .map(|(name, lines)| {
            timed(
                ChartRow::new(name, change(lines), lines.total()),
                spec,
                window.time.model(name),
            )
        })
        .collect();
    Fold::counted(rows, total)
}

fn model_tokens(window: &ActivityWindow, spec: &BlockSpec) -> Fold {
    let spend = window.token_spend();
    let total = spend.iter().map(|(_, billed)| billed).sum();
    let rows = spend
        .into_iter()
        .take(spec.limit)
        .map(|(name, billed)| {
            timed(
                ChartRow::new(name, thousands(billed), billed),
                spec,
                window.time.model(name),
            )
        })
        .collect();
    Fold::plain(rows, total)
}

fn window_time(comparison: &ActivityComparison) -> Fold {
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
    Fold::plain(rows, total)
}

fn window_lines(comparison: &ActivityComparison, spec: &BlockSpec) -> Fold {
    let windows = [&comparison.recent, &comparison.baseline];
    // Spans contain one another, so the largest is the only sensible thing for
    // the shorter ones to read as a share of; summing them would count the
    // recent span twice.
    let total = windows
        .iter()
        .map(|window| written(&window.lines.authors))
        .max_by_key(Lines::total)
        .unwrap_or_default();
    let rows = windows
        .into_iter()
        .map(|window| {
            let row = ChartRow::new(
                window.span.label(),
                change(written(&window.lines.authors)),
                window.lines.total(),
            )
            .divided(
                window.lines.authors.agent.total(),
                window.lines.authors.human.total(),
            );
            timed(row, spec, window.seconds)
        })
        .collect();
    Fold::counted(rows, total)
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
