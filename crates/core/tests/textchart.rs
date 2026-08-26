//! Laying out a chart in characters.
//!
//! These tests are mostly about alignment, which is the one thing a text chart
//! cannot be approximately right about: it will be read in a monospace font, and
//! a column that is a character short in one row makes the whole block look
//! broken. So they check column positions rather than just the presence of words.

use github_personal_stats_core::{
    ActivityMeasure, ActivitySpan, Author, BarGlyphs, BlockSpec, ChartBlock, ChartRow, ChartRows,
    ChartStyle, ChartSummary, ChartValue, Column, DayBucket, MEASURE_AGENT, MEASURE_IMPORTED,
    UNKNOWN_LANGUAGE, build_blocks, compare_activity, default_blocks, format_duration_aligned,
    parse_blocks, render_text_chart,
};

fn block() -> ChartBlock {
    ChartBlock::new("TIME  BY LANGUAGE")
        .with_summary(ChartSummary::new("Total", "91 hrs 46 mins", "last 30 days"))
        .with_rows(
            vec![
                ChartRow::new("Markdown", "45 hrs 23 mins", 163_380),
                ChartRow::new("Go", "41 hrs 13 mins", 148_380),
                ChartRow::new("TypeScript", "5 hrs 10 mins", 18_600),
            ],
            330_360,
        )
}

/// The column positions of every non-space run in a line, which is what a reader
/// actually perceives as alignment.
fn columns(line: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut inside = false;
    for (index, character) in line.chars().enumerate() {
        if character == ' ' {
            inside = false;
        } else if !inside {
            starts.push(index);
            inside = true;
        }
    }
    starts
}

#[test]
fn every_row_puts_its_columns_in_the_same_place() {
    let text = render_text_chart(&[block()], &ChartStyle::default());
    let rows = text
        .lines()
        .filter(|line| {
            line.starts_with("Markdown") || line.starts_with("Go") || line.starts_with("TypeScript")
        })
        .collect::<Vec<_>>();

    assert_eq!(rows.len(), 3, "{text}");

    // The bar and the share start at the same offset in all three rows, which is
    // only true if the name and value columns were padded to their widest cell.
    let first = columns(rows[0]);
    for row in &rows[1..] {
        let held = columns(row);
        assert_eq!(
            held.last(),
            first.last(),
            "the share column moved between rows\n{text}"
        );
    }
}

#[test]
fn a_long_name_widens_the_column_for_every_row() {
    let narrow = render_text_chart(&[block()], &ChartStyle::default());
    let mut wider = block();
    wider.rows[0].name = "ProtocolBuffersDefinition".to_owned();
    let wide = render_text_chart(&[wider], &ChartStyle::default());

    let narrow_go = narrow.lines().find(|line| line.starts_with("Go")).unwrap();
    let wide_go = wide.lines().find(|line| line.starts_with("Go")).unwrap();

    assert!(
        wide_go.len() > narrow_go.len(),
        "a longer name in one row has to push every row's later columns right\n{wide}"
    );
}

#[test]
fn values_line_up_on_their_last_digit() {
    let text = render_text_chart(&[block()], &ChartStyle::default());
    let lines = text
        .lines()
        .filter(|line| line.contains("hrs"))
        .collect::<Vec<_>>();

    let ends = lines
        .iter()
        .map(|line| line.find("hrs").unwrap() + 3)
        .collect::<Vec<_>>();

    assert!(
        ends.windows(2).all(|pair| pair[0] == pair[1]),
        "durations must be right aligned so the hours sit above each other\n{text}"
    );
}

#[test]
fn a_bar_is_as_long_as_the_row_is_large() {
    let style = ChartStyle {
        bar_cells: 10,
        ..ChartStyle::default()
    };
    let text = render_text_chart(&[block()], &style);
    let leader = text.lines().find(|line| line.contains("Markdown")).unwrap();
    let smallest = text
        .lines()
        .find(|line| line.contains("TypeScript"))
        .unwrap();

    assert_eq!(
        leader.matches('#').count(),
        10,
        "the largest row fills the bar\n{text}"
    );
    assert!(
        smallest.matches('#').count() <= 2,
        "a row an eighth the size of the leader gets about an eighth of the bar\n{text}"
    );
}

#[test]
fn a_divided_row_shows_the_split_inside_its_bar() {
    let divided = ChartBlock::new("LINES  BY LANGUAGE")
        .with_rows(
            vec![ChartRow::new("Rust", "1,000", 1_000).divided(750, 250)],
            1_000,
        )
        .divided_into("agent", "not by an agent");
    let style = ChartStyle {
        bar_cells: 20,
        ..ChartStyle::default()
    };
    let text = render_text_chart(&[divided], &style);

    assert!(text.contains("###############====="), "{text}");
    // The legend only appears when something was actually divided, so a chart of
    // durations is not annotated with a split it does not have.
    assert!(text.contains("# agent"), "{text}");
    assert!(!render_text_chart(&[block()], &style).contains("# agent"));

    // A block that divides without saying what its parts are gets no key, since
    // a key naming nothing would be worse than none.
    let unnamed = ChartBlock::new("LINES  BY LANGUAGE").with_rows(
        vec![ChartRow::new("Rust", "1,000", 1_000).divided(750, 250)],
        1_000,
    );
    assert!(!render_text_chart(&[unnamed], &style).contains("agent"));
}

#[test]
fn the_bar_characters_are_a_choice() {
    let style = ChartStyle {
        glyphs: BarGlyphs::parse(">~-").unwrap(),
        bar_cells: 8,
        ..ChartStyle::default()
    };
    let text = render_text_chart(&[block()], &style);

    assert!(text.contains(">>>>>>>>"), "{text}");
    assert!(!text.contains('#'), "{text}");
}

#[test]
fn two_characters_are_accepted_for_a_chart_with_nothing_to_divide() {
    let glyphs = BarGlyphs::parse("*.").unwrap();

    assert_eq!(glyphs.primary, '*');
    assert_eq!(glyphs.secondary, '*');
    assert_eq!(glyphs.empty, '.');
}

#[test]
fn one_character_is_not_enough_to_draw_a_bar_with() {
    assert!(BarGlyphs::parse("#").is_err());
}

#[test]
fn columns_can_be_dropped_and_reordered() {
    let style = ChartStyle {
        columns: vec![Column::Bar, Column::Name],
        bar_cells: 5,
        ..ChartStyle::default()
    };
    let text = render_text_chart(&[block()], &style);
    let leader = text.lines().find(|line| line.contains("Markdown")).unwrap();

    assert!(leader.starts_with("#####"), "{text}");
    assert!(
        !text.contains('%'),
        "a dropped column leaves nothing behind\n{text}"
    );
    assert!(!text.contains("hrs"), "{text}");
}

#[test]
fn a_block_with_no_rows_says_so_rather_than_drawing_nothing() {
    let text = render_text_chart(
        &[ChartBlock::new("TOKENS  BY MODEL")],
        &ChartStyle::default(),
    );

    assert!(text.contains("TOKENS  BY MODEL"), "{text}");
    assert!(text.contains("nothing recorded"), "{text}");
}

#[test]
fn a_spec_reads_as_a_value_a_dimension_and_its_settings() {
    let blocks = parse_blocks("time/languages,limit=3;lines/models,title=Models").unwrap();

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].value, ChartValue::Time);
    assert_eq!(blocks[0].rows, ChartRows::Languages);
    assert_eq!(blocks[0].limit, 3);
    assert_eq!(blocks[1].title, "Models");
}

#[test]
fn a_spec_that_names_nothing_real_is_refused_rather_than_ignored() {
    assert!(parse_blocks("time").is_err(), "a spec needs a dimension");
    assert!(parse_blocks("hours/languages").is_err(), "unknown value");
    assert!(parse_blocks("time/files").is_err(), "unknown dimension");
    assert!(parse_blocks("time/languages,limit=lots").is_err());
    assert!(parse_blocks("time/languages,depth=2").is_err());
}

#[test]
fn the_blocks_a_record_can_fill_are_folded_from_its_facts() {
    let mut day = DayBucket::new("2026-08-25");
    {
        let bucket = day.measure_mut(MEASURE_AGENT);
        bucket.seconds = 7_200;
        bucket.sessions = 2;
        bucket.languages.insert("Rust".to_owned(), 5_400);
        bucket.languages.insert("Markdown".to_owned(), 1_800);
    }
    day.add_lines("Rust", Author::Agent, "gpt-5.6", 800, 0);
    day.add_lines("Rust", Author::Human, "", 200, 0);
    day.add_lines("Markdown", Author::Agent, "claude-opus-5", 300, 0);

    let comparison = compare_activity(
        &[day],
        ActivityMeasure::default(),
        [ActivitySpan::Days(30), ActivitySpan::All],
        Some("2026-08-25"),
        10,
        &[],
    );
    let text = render_text_chart(
        &build_blocks(std::slice::from_ref(&comparison), &default_blocks()),
        &ChartStyle::default(),
    );

    assert!(text.contains("TIME  BY LANGUAGE"), "{text}");
    assert!(text.contains("2 hrs  0 mins"), "{text}");
    assert!(text.contains("LINES  BY AUTHOR"), "{text}");
    // Eleven hundred agent lines against two hundred of mine.
    assert!(text.contains("1,100"), "{text}");
    assert!(text.contains("LINES  BY MODEL"), "{text}");
    assert!(text.contains("gpt-5.6"), "{text}");
}

#[test]
fn a_language_block_of_lines_divides_each_bar_by_author() {
    let mut day = DayBucket::new("2026-08-25");
    day.measure_mut(MEASURE_AGENT).seconds = 3_600;
    day.add_lines("Rust", Author::Agent, "gpt-5.6", 600, 0);
    day.add_lines("Rust", Author::Human, "", 400, 0);

    let comparison = compare_activity(
        &[day],
        ActivityMeasure::default(),
        [ActivitySpan::Days(30), ActivitySpan::All],
        Some("2026-08-25"),
        10,
        &[],
    );
    let blocks = parse_blocks("lines/languages").unwrap();
    let text = render_text_chart(
        &build_blocks(std::slice::from_ref(&comparison), &blocks),
        &ChartStyle::default(),
    );

    // Sixty percent of a full bar in the primary glyph, the rest in the second.
    assert!(text.contains("###############=========="), "{text}");
}

#[test]
fn a_block_whose_data_was_never_recorded_stays_empty_rather_than_guessing() {
    let mut day = DayBucket::new("2026-08-25");
    day.measure_mut(MEASURE_AGENT).seconds = 3_600;
    day.add_lines("Rust", Author::Agent, "gpt-5.6", 600, 0);

    let comparison = compare_activity(
        &[day],
        ActivityMeasure::default(),
        [ActivitySpan::Days(30), ActivitySpan::All],
        Some("2026-08-25"),
        10,
        &[],
    );
    let blocks = parse_blocks("tokens/models").unwrap();
    let text = render_text_chart(
        &build_blocks(std::slice::from_ref(&comparison), &blocks),
        &ChartStyle::default(),
    );

    assert!(text.contains("nothing recorded"), "{text}");
}

#[test]
fn the_total_lines_up_with_the_figures_it_totals() {
    let text = render_text_chart(&[block()], &ChartStyle::default());
    let total = text.lines().find(|line| line.starts_with("Total")).unwrap();
    let row = text
        .lines()
        .find(|line| line.starts_with("Markdown"))
        .unwrap();

    assert_eq!(
        total.find("hrs").map(|at| at + 3),
        row.find("hrs").map(|at| at + 3),
        "the total's duration must sit above the rows' durations\n{text}"
    );
}

#[test]
fn a_long_note_on_the_total_does_not_move_the_percentages() {
    let tight = render_text_chart(&[block()], &ChartStyle::default());
    let mut wordy = block();
    wordy.summary = Some(ChartSummary::new(
        "Total",
        "91 hrs 46 mins",
        "last 30 days, 30 of 30 days, on two machines",
    ));
    let loose = render_text_chart(&[wordy], &ChartStyle::default());

    let percent_at = |text: &str| {
        text.lines()
            .find(|line| line.starts_with("Markdown"))
            .and_then(|line| line.find('%'))
    };

    assert_eq!(
        percent_at(&tight),
        percent_at(&loose),
        "a sentence on the total line is not part of the table\n{loose}"
    );
}

#[test]
fn a_blocks_total_is_the_total_of_what_it_shows() {
    let mut day = DayBucket::new("2026-08-25");
    day.measure_mut(MEASURE_AGENT).seconds = 3_600;
    day.add_lines("Rust", Author::Agent, "gpt-5.6", 900, 0);
    day.add_lines("Rust", Author::Human, "", 100, 0);

    let comparison = compare_activity(
        &[day],
        ActivityMeasure::default(),
        [ActivitySpan::Days(30), ActivitySpan::All],
        Some("2026-08-25"),
        10,
        &[],
    );
    let by_author = render_text_chart(
        &build_blocks(
            std::slice::from_ref(&comparison),
            &parse_blocks("lines/authors").unwrap(),
        ),
        &ChartStyle::default(),
    );
    let by_model = render_text_chart(
        &build_blocks(
            std::slice::from_ref(&comparison),
            &parse_blocks("lines/models").unwrap(),
        ),
        &ChartStyle::default(),
    );

    // Everyone wrote a thousand lines; the models wrote nine hundred of them. A
    // block that totalled the wrong one would make its own percentages read wrong.
    assert!(
        by_author
            .lines()
            .any(|line| line.starts_with("Total") && line.contains("1,000")),
        "{by_author}"
    );
    assert!(by_model.contains("Total"), "{by_model}");
    assert!(by_model.contains("900"), "{by_model}");
    assert!(!by_model.contains("1,000"), "{by_model}");
}

#[test]
fn minutes_line_up_even_when_one_row_has_a_single_digit() {
    let hours = ChartBlock::new("TIME  BY LANGUAGE").with_rows(
        vec![
            ChartRow::new("Clojure", format_duration_aligned(5_901_180), 5_901_180),
            ChartRow::new("Go", format_duration_aligned(3_431_220), 3_431_220),
        ],
        9_332_400,
    );
    let text = render_text_chart(&[hours], &ChartStyle::default());
    let lines = text
        .lines()
        .filter(|line| line.contains("mins"))
        .collect::<Vec<_>>();

    // Seven minutes and thirteen minutes have to put their last digit in the
    // same column, which right aligning the whole phrase does not achieve.
    let digits = lines
        .iter()
        .map(|line| line.find(" mins").unwrap())
        .collect::<Vec<_>>();

    assert!(
        digits.windows(2).all(|pair| pair[0] == pair[1]),
        "the minute digits wander\n{text}"
    );
    assert!(text.contains("1,639 hrs 13 mins"), "{text}");
    assert!(text.contains("953 hrs  7 mins"), "{text}");
}

#[test]
fn a_long_duration_groups_its_hours() {
    assert_eq!(format_duration_aligned(25_703_112), "7,139 hrs 45 mins");
}

#[test]
fn a_block_reads_the_measure_it_names_rather_than_the_charts() {
    let mut day = DayBucket::new("2026-08-20");
    day.measure_mut(MEASURE_AGENT).seconds = 3_600;
    day.measure_mut(MEASURE_IMPORTED).seconds = 28_800;
    let days = vec![day];

    let fold = |measure: &str| {
        compare_activity(
            &days,
            ActivityMeasure::new(measure),
            [ActivitySpan::Days(30), ActivitySpan::All],
            None,
            12,
            &[],
        )
    };
    let folds = vec![fold(MEASURE_AGENT), fold(MEASURE_IMPORTED)];

    let specs = vec![
        BlockSpec::new(ChartValue::Time, ChartRows::Windows).of(MEASURE_AGENT),
        BlockSpec::new(ChartValue::Time, ChartRows::Windows).of(MEASURE_IMPORTED),
    ];
    let text = render_text_chart(&build_blocks(&folds, &specs), &ChartStyle::default());

    // Hours an agent spent and hours carried in from elsewhere overlap, so a
    // chart may put them side by side under names that say which is which.
    assert!(text.contains("AGENT TIME  BY SPAN"), "{text}");
    assert!(text.contains("IMPORTED TIME  BY SPAN"), "{text}");
    assert!(text.contains("1 hrs  0 mins"), "{text}");
    assert!(text.contains("8 hrs  0 mins"), "{text}");
}

#[test]
fn a_block_naming_a_measure_nobody_folded_stays_empty() {
    let folds = vec![compare_activity(
        &[DayBucket::new("2026-08-20")],
        ActivityMeasure::new(MEASURE_AGENT),
        [ActivitySpan::Days(30), ActivitySpan::All],
        None,
        12,
        &[],
    )];
    let specs = vec![BlockSpec::new(ChartValue::Time, ChartRows::Windows).of("treadmill")];
    let text = render_text_chart(&build_blocks(&folds, &specs), &ChartStyle::default());

    assert!(text.contains("TREADMILL TIME  BY SPAN"), "{text}");
    assert!(text.contains("nothing recorded"), "{text}");
}

#[test]
fn dropping_the_bar_does_not_drop_what_the_total_is_a_total_of() {
    let block = ChartBlock::new("TIME  BY LANGUAGE")
        .with_summary(ChartSummary::new(
            "Total",
            "9 hrs",
            "last 30 days, 4 of 30 days",
        ))
        .with_rows(vec![ChartRow::new("Rust", "9 hrs", 9)], 9);
    let bare = ChartStyle {
        columns: vec![Column::Name, Column::Value],
        ..ChartStyle::default()
    };
    let text = render_text_chart(&[block], &bare);

    // The note usually rides in the bar's column; without one it has to trail
    // the row rather than vanish.
    assert!(text.contains("last 30 days, 4 of 30 days"), "{text}");
}

#[test]
fn lines_whose_language_went_unrecorded_are_named_rather_than_dropped() {
    let mut day = DayBucket::new("2026-08-20");
    day.measure_mut(MEASURE_AGENT).seconds = 3_600;
    day.add_lines("Rust", Author::Agent, "a-model", 400, 0);
    day.add_lines(UNKNOWN_LANGUAGE, Author::Agent, "a-model", 100, 0);

    let fold = compare_activity(
        &[day],
        ActivityMeasure::new(MEASURE_AGENT),
        [ActivitySpan::Days(30), ActivitySpan::All],
        None,
        12,
        &[],
    );
    let specs = vec![
        BlockSpec::new(ChartValue::Lines, ChartRows::Languages),
        BlockSpec::new(ChartValue::Lines, ChartRows::Authors),
    ];
    let text = render_text_chart(
        &build_blocks(std::slice::from_ref(&fold), &specs),
        &ChartStyle::default(),
    );

    // A file without a telling extension still has lines in it. Leaving them out
    // of the per-language block would make it disagree with the block beside it.
    assert!(text.contains("unknown"), "{text}");
    let totals = text
        .lines()
        .filter(|line| line.starts_with("Total"))
        .map(|line| {
            line.split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(totals, vec!["500", "500"], "the two totals differ\n{text}");
}

#[test]
fn a_time_block_divides_its_bars_by_who_spent_the_time() {
    let mut day = DayBucket::new("2026-08-20");
    let bucket = day.measure_mut(MEASURE_AGENT);
    bucket.seconds = 4_000;
    bucket.languages.insert("Rust".to_owned(), 4_000);
    bucket.spend("Rust", Author::Agent, 3_000);
    bucket.spend("Rust", Author::Human, 1_000);

    let fold = compare_activity(
        &[day],
        ActivityMeasure::new(MEASURE_AGENT),
        [ActivitySpan::Days(30), ActivitySpan::All],
        None,
        12,
        &[],
    );
    let text = render_text_chart(
        &build_blocks(
            std::slice::from_ref(&fold),
            &parse_blocks("time/languages,split=on").unwrap(),
        ),
        &ChartStyle {
            bar_cells: 20,
            ..ChartStyle::default()
        },
    );

    // Three quarters of the hour was an agent's and a quarter was not, by the
    // same rule that put the hour against Rust in the first place.
    assert!(text.contains("###############====="), "{text}");
    assert!(text.contains("# agent"), "{text}");
}

#[test]
fn time_from_a_source_that_names_no_author_is_drawn_in_one_piece() {
    let mut day = DayBucket::new("2026-08-20");
    let bucket = day.measure_mut(MEASURE_IMPORTED);
    bucket.seconds = 4_000;
    bucket.languages.insert("Clojure".to_owned(), 4_000);

    let fold = compare_activity(
        &[day],
        ActivityMeasure::new(MEASURE_IMPORTED),
        [ActivitySpan::Days(30), ActivitySpan::All],
        None,
        12,
        &[],
    );
    let text = render_text_chart(
        &build_blocks(
            std::slice::from_ref(&fold),
            &parse_blocks("time/languages,split=on").unwrap(),
        ),
        &ChartStyle {
            bar_cells: 20,
            ..ChartStyle::default()
        },
    );

    // An imported hour is an hour nobody claimed, which is not the same as an
    // hour claimed by nobody: the bar is whole and there is no key.
    assert!(text.contains("####################"), "{text}");
    assert!(!text.contains("="), "{text}");
    assert!(!text.contains("# agent"), "{text}");
}
