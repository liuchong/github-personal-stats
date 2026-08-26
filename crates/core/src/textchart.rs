//! Laying out a chart in text, aligned so it reads as a table.
//!
//! A chart drawn in characters has one hard requirement and one temptation. The
//! requirement is alignment: it will be read in a monospace font, so every column
//! has to be as wide as its widest cell or the numbers wander and the block stops
//! looking like a table. The temptation is to decide, here, what a chart is
//! about — that it shows languages, that it shows hours, that the bar means what
//! this project happens to measure.
//!
//! This module resists that. It takes rows that already know their name, their
//! value, and how that value divides, and it lays them out. What the rows are, how
//! they were folded, and what the numbers mean are settled before anything gets
//! here. That is what makes a new kind of chart a matter of building different
//! rows rather than changing this code.

use std::fmt::Write as _;

/// The characters a bar is drawn with.
///
/// Three of them, because a bar can carry a division: the part of the value one
/// author wrote, the part another did, and the space neither fills. A chart whose
/// value cannot be divided uses only the first and the last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarGlyphs {
    pub primary: char,
    pub secondary: char,
    pub empty: char,
}

impl Default for BarGlyphs {
    /// Plain ASCII, which aligns in every monospace font. Unicode block elements
    /// look better where they render at one cell wide, and several common fonts
    /// do not render them that way, so they are a choice rather than the default.
    fn default() -> Self {
        Self {
            primary: '#',
            secondary: '=',
            empty: '-',
        }
    }
}

impl BarGlyphs {
    /// Reads three characters, in the order primary, secondary, empty.
    pub fn parse(value: &str) -> Result<Self, String> {
        let characters = value.chars().collect::<Vec<_>>();
        match characters.as_slice() {
            [primary, secondary, empty] => Ok(Self {
                primary: *primary,
                secondary: *secondary,
                empty: *empty,
            }),
            // Two is a reasonable thing to write for a chart with nothing to
            // divide, and it would be unkind to reject it.
            [primary, empty] => Ok(Self {
                primary: *primary,
                secondary: *primary,
                empty: *empty,
            }),
            _ => Err(format!(
                "expected two or three characters for fill, split, and empty, got {value:?}"
            )),
        }
    }
}

/// A column of a chart, and the order they appear in is the order they are given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    /// What the row is: a language, a model, an author.
    Name,
    /// The row's figure, already worded by whoever built the row.
    Value,
    /// The bar.
    Bar,
    /// The row's share of the block, as a percentage.
    Share,
}

impl Column {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim() {
            "name" => Ok(Self::Name),
            "value" => Ok(Self::Value),
            "bar" => Ok(Self::Bar),
            "share" => Ok(Self::Share),
            other => Err(format!(
                "unknown column {other:?}; expected name, value, bar, or share"
            )),
        }
    }
}

pub const DEFAULT_COLUMNS: [Column; 4] = [Column::Name, Column::Value, Column::Bar, Column::Share];
pub const DEFAULT_BAR_CELLS: usize = 25;
const GUTTER: usize = 3;

/// How a chart is drawn, as opposed to what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartStyle {
    pub columns: Vec<Column>,
    pub bar_cells: usize,
    pub glyphs: BarGlyphs,
    /// Whether a bar's length is its share of the block or its share of the
    /// largest row.
    ///
    /// Against the largest row by default, because shares of a block are usually
    /// small: a leading language at twenty-eight percent would leave every bar in
    /// the left quarter of its column with nothing in the rest, which wastes the
    /// width without telling the reader anything the percentage has not.
    pub relative_to_largest: bool,
    /// Whether to print a line saying what the bar characters mean.
    pub legend: bool,
}

impl Default for ChartStyle {
    fn default() -> Self {
        Self {
            columns: DEFAULT_COLUMNS.to_vec(),
            bar_cells: DEFAULT_BAR_CELLS,
            glyphs: BarGlyphs::default(),
            relative_to_largest: true,
            legend: true,
        }
    }
}

/// One row: what it is, what it measures, and how that divides.
///
/// `primary` and `secondary` are parts of `value` and need not cover it; what
/// they leave over is drawn as unattributed. A row with nothing to divide leaves
/// both at zero and the bar is drawn in one piece.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChartRow {
    pub name: String,
    /// The figure as it should read, worded by whoever built the row, because
    /// only they know whether it is hours or lines or tokens.
    pub value: String,
    /// What the bar's length and the share are computed from.
    pub weight: u64,
    pub primary: u64,
    pub secondary: u64,
}

impl ChartRow {
    pub fn new(name: impl Into<String>, value: impl Into<String>, weight: u64) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            weight,
            primary: 0,
            secondary: 0,
        }
    }

    /// Divides the row between two authors.
    pub fn divided(mut self, primary: u64, secondary: u64) -> Self {
        self.primary = primary;
        self.secondary = secondary;
        self
    }

    fn divides(&self) -> bool {
        self.primary > 0 || self.secondary > 0
    }
}

/// The block's total, drawn above the rows in the same columns.
///
/// It shares the rows' column widths rather than being written as free text, so
/// the total sits directly above the figures it is the total of. A total that
/// does not line up with them invites the reader to check the arithmetic by eye
/// and makes it hard.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChartSummary {
    pub label: String,
    pub value: String,
    /// What qualifies the total: the span it covers, how much of it was worked.
    pub note: String,
}

impl ChartSummary {
    pub fn new(
        label: impl Into<String>,
        value: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            note: note.into(),
        }
    }
}

/// A heading, a total, some rows, and what they are shares of.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChartBlock {
    pub title: String,
    pub summary: Option<ChartSummary>,
    pub rows: Vec<ChartRow>,
    /// What each row's share is a share of. Zero means shares are not shown, and
    /// summing the rows instead would be wrong for a block whose rows are only
    /// the largest few.
    pub total: u64,
}

impl ChartBlock {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn with_summary(mut self, summary: ChartSummary) -> Self {
        self.summary = Some(summary);
        self
    }

    pub fn with_rows(mut self, rows: Vec<ChartRow>, total: u64) -> Self {
        self.rows = rows;
        self.total = total;
        self
    }
}

/// Lays out blocks as aligned text.
///
/// Blocks are aligned one at a time rather than all together, because a block of
/// language names and a block of model names have little to do with each other
/// and padding the short names out to the width of the long ones would leave a
/// gutter wide enough to lose the reader in.
pub fn render_text_chart(blocks: &[ChartBlock], style: &ChartStyle) -> String {
    let mut out = String::new();
    let mut divided = false;

    for block in blocks {
        if !out.is_empty() {
            out.push('\n');
        }
        if !block.title.is_empty() {
            let _ = writeln!(out, "{}", block.title);
            out.push('\n');
        }
        if block.rows.is_empty() {
            let _ = writeln!(out, "nothing recorded");
            continue;
        }
        divided |= block.rows.iter().any(ChartRow::divides);
        out.push_str(&lay_out(block, style));
    }

    if style.legend && divided {
        let _ = write!(
            out,
            "\n{} agent    {} me    {} rest\n",
            style.glyphs.primary, style.glyphs.secondary, style.glyphs.empty
        );
    }

    out
}

/// The alignment: each column is measured across every row of the block, then
/// every row is padded to those widths.
fn lay_out(block: &ChartBlock, style: &ChartStyle) -> String {
    let cells = block
        .rows
        .iter()
        .map(|row| {
            style
                .columns
                .iter()
                .map(|column| match column {
                    Column::Name => row.name.clone(),
                    Column::Value => row.value.clone(),
                    Column::Bar => bar(row, block, style),
                    Column::Share => share(row.weight, block.total),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    // The total is measured with the rows so that it lines up with them, but it
    // has no bar and no share of itself; its note goes where the bar would be.
    let summary = block.summary.as_ref().map(|summary| {
        style
            .columns
            .iter()
            .map(|column| match column {
                Column::Name => summary.label.clone(),
                Column::Value => summary.value.clone(),
                Column::Bar => summary.note.clone(),
                Column::Share => String::new(),
            })
            .collect::<Vec<_>>()
    });
    let widths = (0..style.columns.len())
        .map(|index| {
            // The total's label and figure are measured with the rows, so they
            // sit above them. Its note is not: the note is a sentence, and
            // letting a long one widen the bar column would push the percentages
            // away from the bars for the sake of a line that has no percentage.
            let measured = summary
                .iter()
                .filter(|_| !matches!(style.columns[index], Column::Bar))
                .chain(cells.iter());
            measured
                .map(|row| row[index].chars().count())
                .max()
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();

    let mut out = String::new();
    if let Some(summary) = &summary {
        let mut line = pad_row(summary, &widths, style);
        // The note rides in the bar's column, so a chart drawn without bars has
        // nowhere to put it and would drop what the figures are a total of. It
        // trails the row instead, where nothing lines up under it anyway.
        if let (None, Some(note)) = (
            style
                .columns
                .iter()
                .position(|column| matches!(column, Column::Bar)),
            block.summary.as_ref().map(|summary| summary.note.as_str()),
        ) && !note.is_empty()
        {
            let _ = write!(line, "{}{note}", " ".repeat(GUTTER));
        }
        let _ = writeln!(out, "{}", line.trim_end());
        out.push('\n');
    }
    for row in &cells {
        let _ = writeln!(out, "{}", pad_row(row, &widths, style));
    }
    out
}

fn pad_row(row: &[String], widths: &[usize], style: &ChartStyle) -> String {
    let mut line = String::new();
    for (index, column) in style.columns.iter().enumerate() {
        if index > 0 {
            line.push_str(&" ".repeat(GUTTER));
        }
        let text = &row[index];
        let pad = widths[index].saturating_sub(text.chars().count());
        // Names and bars read left to right; figures line up on their last digit.
        // A summary's note sits in the bar's column and is prose, so it is left
        // aligned for the same reason a name is.
        let numeric = matches!(column, Column::Value | Column::Share);
        if numeric {
            line.push_str(&" ".repeat(pad));
            line.push_str(text);
        } else {
            line.push_str(text);
            line.push_str(&" ".repeat(pad));
        }
    }
    line.trim_end().to_owned()
}

fn bar(row: &ChartRow, block: &ChartBlock, style: &ChartStyle) -> String {
    let cells = style.bar_cells;
    let against = if style.relative_to_largest {
        block
            .rows
            .iter()
            .map(|row| row.weight)
            .max()
            .unwrap_or_default()
    } else {
        block.total
    };
    if against == 0 || cells == 0 {
        return style.glyphs.empty.to_string().repeat(cells);
    }

    let filled = scale(row.weight, against, cells).min(cells);
    if !row.divides() {
        let mut bar = style.glyphs.primary.to_string().repeat(filled);
        bar.push_str(&style.glyphs.empty.to_string().repeat(cells - filled));
        return bar;
    }

    // The division is drawn inside the filled part, so the bar's length still
    // reads as the row's size and the split reads as the split.
    let known = row.primary + row.secondary;
    let primary = if known == 0 {
        filled
    } else {
        scale(row.primary, known, filled).min(filled)
    };
    let mut bar = style.glyphs.primary.to_string().repeat(primary);
    bar.push_str(&style.glyphs.secondary.to_string().repeat(filled - primary));
    bar.push_str(&style.glyphs.empty.to_string().repeat(cells - filled));
    bar
}

/// Rounds `part / whole * cells` without leaving through a float, so the same
/// figures produce the same bar on every machine.
fn scale(part: u64, whole: u64, cells: usize) -> usize {
    if whole == 0 {
        return 0;
    }
    let cells = u64::try_from(cells).unwrap_or(u64::MAX);
    let scaled = (part.saturating_mul(cells).saturating_mul(2) / whole).saturating_add(1) / 2;
    usize::try_from(scaled).unwrap_or(usize::MAX)
}

fn share(part: u64, whole: u64) -> String {
    if whole == 0 {
        return String::new();
    }
    let hundredths = part.saturating_mul(10_000) / whole;
    format!("{}.{:02} %", hundredths / 100, hundredths % 100)
}
