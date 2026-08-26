use github_personal_stats_core::{
    ActivityComparison, ActivityMeasure, ActivitySpan, BarGlyphs, BlockSpec, ChartStyle, Column,
    DEFAULT_ACTIVITY_WINDOWS, DEFAULT_BAR_CELLS, DEFAULT_HEAT_THRESHOLD, DEFAULT_LANGUAGE_ROWS,
    GithubData, GithubGraphqlClient, GithubStatsConfig, MAX_LANGUAGE_ROWS, MAX_PADDING,
    MockGithubClient, OutputKind, aggregate_card_data, build_blocks, compare_activity,
    default_blocks, json::write_github_fixture, parse_blocks, parse_output_kind, render_card,
    render_text_chart, store::read_record, workspace_info,
};
use std::{env, error::Error, fs, path::PathBuf};

const DEFAULT_CARD: &str = "dashboard";
const DEFAULT_OUTPUT: &str = "profile/github-personal-stats.svg";
const DEFAULT_DATA: &str = "github-personal-stats.json";
const DEFAULT_USER: &str = "octo";
const DEFAULT_WIDTH: u32 = 1000;
const DEFAULT_HEIGHT: u32 = 420;
const DEFAULT_THEME: &str = "light";
const DEFAULT_TARGET: &str = "README.md";
const DEFAULT_SECTION: &str = "activity";
const DEFAULT_STAT_ROWS: &str = "stars,commits,prs,issues";
const DEFAULT_METRIC: &str = "current";

/// Cards whose content decides their height. The dashboard and the status cards
/// divide a height they are given, so fitting them to content is meaningless.
const AUTO_HEIGHT_CARDS: [OutputKind; 5] = [
    OutputKind::Stats,
    OutputKind::Languages,
    OutputKind::Streak,
    OutputKind::Heat,
    OutputKind::Metric,
];
const DEFAULT_STREAK_SIDES: &str = "total,longest";

/// How many languages the fold keeps. Generous on purpose: a chart block trims to
/// its own limit, and trimming earlier would decide the ranking for every block
/// by whichever one the fold happened to sort by.
const FOLD_LANGUAGES: usize = 200;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let command = if args.is_empty() {
        "info".to_owned()
    } else {
        args.remove(0)
    };

    if is_help(&command) || args.iter().any(|arg| is_help(arg)) {
        println!("{}", usage());
        return Ok(());
    }

    match command.as_str() {
        "info" => println!("{}", workspace_info().to_json()),
        "fetch" => fetch(args)?,
        "generate" => generate(args)?,
        "chart" => chart(args)?,
        "update-readme" => update_readme(args)?,
        command => {
            return Err(format!("unsupported command: {command}\n\n{}", usage()).into());
        }
    }

    Ok(())
}

fn is_help(argument: &str) -> bool {
    matches!(argument, "help" | "--help" | "-h")
}

fn usage() -> String {
    format!(
        "github-personal-stats <command> [options]

Commands:
  fetch                   Read a profile once and save it for later renders
  generate                Render a card to an SVG file
  chart                   Render collected activity as an aligned text chart
  update-readme           Rewrite a marked section of a README
  info                    Print workspace information as JSON
  help, --help, -h        Print this message

Fetch options:
  --user <login>          Profile to read (default: {DEFAULT_USER})
  --output <path>         Where to save the profile (default: {DEFAULT_DATA})
  --authored-languages, --author-email, --min-repo-language-share
                          Read as they are for generate; the answers they give
                          are what gets saved

Generate options:
  --user <login>          Profile to read (default: {DEFAULT_USER})
  --card <kind>           dashboard, stats, languages, streak, heat, metric,
                          activity, status (default: {DEFAULT_CARD})
  --output <path>         Where to write the card (default: {DEFAULT_OUTPUT})
  --width <pixels>        Card width (default: {DEFAULT_WIDTH})
  --height <pixels|auto>  Card height, or auto to fit the content of a stats,
                          languages, streak, heat, or metric card
                          (default: {DEFAULT_HEIGHT})
  --theme <name>          light, dark, transparent (default: {DEFAULT_THEME})
  --padding <pixels|auto> Inner margin; pin it to align tiles of differing
                          widths (default: auto, {MAX_PADDING} at most)
  --scale <multiplier>    Display the card larger or smaller than it is laid
                          out, without redrawing it (default: 1)
  --fixture <path>        Read a saved profile instead of the network, whether
                          written by fetch or by hand
  --authored-languages    Count only repositories the profile contributed to
  --author-email <email>  Extra commit email for authorship matching, repeatable
  --hide-language <name>  Language to leave out, repeatable
  --min-repo-language-share <percent>
                          Ignore a language below this share of one repository

Panel content options:
  --stat-rows <list>      Ordered stats rows from stars, commits, prs, issues,
                          reviews, repos (default: {DEFAULT_STAT_ROWS})
  --language-rows <count> Languages to list, 1 to {MAX_LANGUAGE_ROWS} (default: {DEFAULT_LANGUAGE_ROWS})
  --streak-sides <left,right>
                          Figures beside the ring from total, longest, current,
                          active (default: {DEFAULT_STREAK_SIDES})
  --metric <name>         Figure drawn by --card metric, from any of the stats
                          or streak names above (default: {DEFAULT_METRIC})

Heat ring options:
  --heat-window <streak|days>
                          Days the ring covers (default: streak)
  --heat-limit <days|none>
                          Cap the ring without shortening the streak (default: none)
  --heat-shape <shape>    segmented, ticks, arcs, bands (default: segmented)
  --heat-threshold <days> Where ticks become arcs (default: {DEFAULT_HEAT_THRESHOLD})
  --heat-scale <scale>    linear, sqrt, log, quantile (default: linear)
  --heat-color <palette>  heat-orange, github-blue, forest, violet, crimson,
                          graphite, one hex value, or four hex values
                          (default: heat-orange)
  --heat-label <template> Ring centre text over {{X}} active days, {{Y}} window
                          days, and {{Z}} the streak in full

Activity options:
  --activity-record <dir> Directory of collected activity, one subdirectory per
                          machine holding dated day files
  --activity-measure <name>
                          Which measure of time to report: agent, editor,
                          imported, or any name in the record (default: agent)
  --activity-windows <recent,baseline>
                          Two spans to compare, each a day count or all
                          (default: 30,all)
  --activity-blocks <spec>
                          Text chart blocks, semicolon separated, each written
                          value/dimension with optional settings. Values are
                          time, lines, tokens; dimensions are languages, models,
                          authors, windows; settings are limit, split, title,
                          measure. A block naming a measure reads that one, so
                          one chart can hold agent hours beside imported hours
                          (default: time/languages;lines/authors;lines/models)
  --activity-columns <list>
                          Columns in order from name, value, bar, share
                          (default: name,value,bar,share)
  --activity-bar <chars>  Two or three characters for the agent's share, the
                          share no agent wrote, and empty (default: #=-)
  --activity-bar-width <cells>
                          Bar width in characters (default: {DEFAULT_BAR_CELLS})
  --activity-bar-basis <largest|total>
                          Whether a bar's length is measured against the biggest
                          row or the block total (default: largest)

Update-readme options:
  --target <path>         README to rewrite (default: {DEFAULT_TARGET})
  --section <name>        Marker name to replace (default: {DEFAULT_SECTION})

Environment:
  GITHUB_TOKEN            Token used for live GitHub data

Card aliases top-langs, top-languages, and coding-activity are accepted.
Ring options are illustrated in docs/user-guide.md.

Rendering never asks GitHub anything, so a set of tiles costs one fetch:
  github-personal-stats fetch --user octo --output octo.json
  github-personal-stats generate --fixture octo.json --card stats  --output stats.svg
  github-personal-stats generate --fixture octo.json --card heat   --output heat.svg"
    )
}

fn fetch(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let user = option_value(&args, "--user").unwrap_or_else(|| DEFAULT_USER.to_owned());
    let output = option_value(&args, "--output").unwrap_or_else(|| DEFAULT_DATA.to_owned());
    let config = with_fetch_options(GithubStatsConfig::new(user)?, &args)?;
    let data = live_github_data(&config)?;

    write_output(PathBuf::from(output), write_github_fixture(&data))?;
    Ok(())
}

/// Options that change what GitHub is asked for rather than what is drawn. Their
/// answers are what a saved profile holds, which is why passing them next to a
/// saved profile is an error rather than a quiet no-op.
const FETCH_OPTIONS: [&str; 3] = [
    "--authored-languages",
    "--author-email",
    "--min-repo-language-share",
];

/// Saving a profile saves the answers to [`FETCH_OPTIONS`], so `fetch` and
/// `generate` have to read them the same way.
fn with_fetch_options(
    mut config: GithubStatsConfig,
    args: &[String],
) -> Result<GithubStatsConfig, Box<dyn Error>> {
    if option_flag(args, "--authored-languages") {
        config = config.with_authored_languages();
    }
    config = config.with_author_emails(option_values(args, "--author-email"));
    if let Some(value) = option_value(args, "--min-repo-language-share") {
        config = config.with_min_repo_language_share(&value)?;
    }
    Ok(config)
}

fn generate(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let card = option_value(&args, "--card").unwrap_or_else(|| DEFAULT_CARD.to_owned());
    let output = option_value(&args, "--output").unwrap_or_else(|| DEFAULT_OUTPUT.to_owned());
    let user = option_value(&args, "--user").unwrap_or_else(|| DEFAULT_USER.to_owned());
    let width = option_value(&args, "--width")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(DEFAULT_WIDTH);
    let requested_height = option_value(&args, "--height");
    let auto_height = requested_height
        .as_deref()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("auto"));
    let height = requested_height
        .as_deref()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(DEFAULT_HEIGHT);
    let mut config = GithubStatsConfig::new(user)?.with_size(width, height)?;
    if auto_height {
        if !AUTO_HEIGHT_CARDS.contains(&parse_output_kind(&card)?) {
            return Err(format!(
                "auto height fits a card to its content, which only makes sense for {}; \
                 give {card} an explicit height",
                AUTO_HEIGHT_CARDS
                    .iter()
                    .map(|kind| kind.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .into());
        }
        config = config.with_auto_height();
    }
    if let Some(value) = option_value(&args, "--theme") {
        config = config.with_theme(&value)?;
    }
    if let Some(value) = option_value(&args, "--padding") {
        config = config.with_padding(&value)?;
    }
    if let Some(value) = option_value(&args, "--scale") {
        config = config.with_scale(&value)?;
    }
    config = with_fetch_options(config, &args)?;
    config = config.with_hidden_languages(option_values(&args, "--hide-language"));
    if let Some(value) = option_value(&args, "--stat-rows") {
        config = config.with_stat_rows(&value)?;
    }
    if let Some(value) = option_value(&args, "--language-rows") {
        config = config.with_language_rows(&value)?;
    }
    if let Some(value) = option_value(&args, "--streak-sides") {
        config = config.with_streak_sides(&value)?;
    }
    if let Some(value) = option_value(&args, "--metric") {
        config = config.with_metric(&value)?;
    }
    if let Some(value) = option_value(&args, "--heat-window") {
        config = config.with_heat_window(&value)?;
    }
    if let Some(value) = option_value(&args, "--heat-limit") {
        config = config.with_heat_limit(&value)?;
    }
    if let Some(value) = option_value(&args, "--heat-shape") {
        config = config.with_heat_shape(&value)?;
    }
    if let Some(value) = option_value(&args, "--heat-threshold") {
        config = config.with_heat_threshold(&value)?;
    }
    if let Some(value) = option_value(&args, "--heat-scale") {
        config = config.with_heat_scale(&value)?;
    }
    if let Some(value) = option_value(&args, "--heat-color") {
        config = config.with_heat_color(&value)?;
    }
    if let Some(value) = option_value(&args, "--heat-label") {
        config = config.with_heat_label(&value);
    }
    let saved = option_value(&args, "--fixture");
    if saved.is_some() {
        let already_answered = FETCH_OPTIONS
            .into_iter()
            .filter(|option| args.iter().any(|argument| argument == option))
            .collect::<Vec<_>>();
        if !already_answered.is_empty() {
            return Err(format!(
                "{} decide what to ask GitHub for, and a saved profile has been asked already; \
                 pass them to fetch instead",
                already_answered.join(" and ")
            )
            .into());
        }
    }
    let mut data = github_data(&config, saved)?;
    hide_languages(&mut data, &config.hidden_languages);
    let card_data = aggregate_card_data(&data, parse_output_kind(&card)?, &config.heat_ring);
    let rendered = render_card(&card_data, &config);

    write_output(PathBuf::from(output), rendered)?;
    Ok(())
}

/// Renders the collected activity as text.
fn chart(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let text = activity_chart(&args)?;
    match option_value(&args, "--output") {
        Some(path) => write_output(PathBuf::from(path), text)?,
        None => println!("{}", text.trim_end()),
    }
    Ok(())
}

fn update_readme(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let target =
        PathBuf::from(option_value(&args, "--target").unwrap_or_else(|| DEFAULT_TARGET.to_owned()));
    let section = option_value(&args, "--section").unwrap_or_else(|| DEFAULT_SECTION.to_owned());
    let start = format!("<!--START_SECTION:{section}-->");
    let end = format!("<!--END_SECTION:{section}-->");
    let source = fs::read_to_string(&target)?;

    // A record turns into the chart it describes. Without one there is nothing to
    // say: what used to stand in here was two invented figures, which a reader
    // had no way of telling from the real thing.
    let replacement = format!("```txt\n{}\n```", activity_chart(&args)?.trim_end());
    let updated = replace_section(&source, &start, &end, &replacement)?;

    fs::write(target, updated)?;
    Ok(())
}

/// Reads the record, folds it, and lays out the chart described by the arguments.
fn activity_chart(args: &[String]) -> Result<String, Box<dyn Error>> {
    let blocks = match option_value(args, "--activity-blocks") {
        Some(spec) => parse_blocks(&spec)?,
        None => default_blocks(),
    };
    Ok(render_text_chart(
        &build_blocks(&activity_folds(args, &blocks)?, &blocks),
        &chart_style(args)?,
    ))
}

/// One fold per measure the chart reads: the chart's own, then any a block asked
/// for by name. The record is read once and folded repeatedly, since folding is
/// arithmetic over days already in hand while reading is thousands of files.
fn activity_folds(
    args: &[String],
    blocks: &[BlockSpec],
) -> Result<Vec<ActivityComparison>, Box<dyn Error>> {
    let mut measures = vec![activity_measure(args)?];
    for block in blocks {
        let named = block.measure.trim();
        if !named.is_empty() && !measures.iter().any(|held| held.as_str() == named) {
            measures.push(ActivityMeasure::new(named));
        }
    }
    activity_comparisons(args, measures)
}

fn activity_measure(args: &[String]) -> Result<ActivityMeasure, Box<dyn Error>> {
    Ok(match option_value(args, "--activity-measure") {
        Some(name) => ActivityMeasure::new(name.trim()),
        None => ActivityMeasure::default(),
    })
}

fn activity_comparisons(
    args: &[String],
    measures: Vec<ActivityMeasure>,
) -> Result<Vec<ActivityComparison>, Box<dyn Error>> {
    let root = option_value(args, "--activity-record")
        .ok_or("--activity-record is needed to say where the collected activity lives")?;
    let days = read_record(&PathBuf::from(root))?;

    let windows = match option_value(args, "--activity-windows") {
        Some(value) => parse_windows(&value)?,
        None => DEFAULT_ACTIVITY_WINDOWS,
    };
    let hidden = option_values(args, "--hide-language");

    // The fold keeps far more languages than any block will draw, so a block's
    // own limit is what trims the chart. Trimming here instead would let a
    // language be dropped for being short on time before a block that ranks by
    // lines ever saw it.
    Ok(measures
        .into_iter()
        .map(|measure| compare_activity(&days, measure, windows, None, FOLD_LANGUAGES, &hidden))
        .collect())
}

fn parse_windows(value: &str) -> Result<[ActivitySpan; 2], Box<dyn Error>> {
    let written = value.split(',').map(str::trim).collect::<Vec<_>>();
    let [recent, baseline] = written.as_slice() else {
        return Err(format!(
            "--activity-windows takes two spans separated by a comma, such as 30,all; got {value:?}"
        )
        .into());
    };
    let read = |span: &str| {
        ActivitySpan::parse(span)
            .ok_or_else(|| format!("--activity-windows span {span:?} must be a day count or all"))
    };
    Ok([read(recent)?, read(baseline)?])
}

fn chart_style(args: &[String]) -> Result<ChartStyle, Box<dyn Error>> {
    let mut style = ChartStyle::default();
    if let Some(value) = option_value(args, "--activity-columns") {
        style.columns = value
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(Column::parse)
            .collect::<Result<Vec<_>, _>>()?;
        if style.columns.is_empty() {
            return Err("--activity-columns needs at least one column".into());
        }
    }
    if let Some(value) = option_value(args, "--activity-bar") {
        style.glyphs = BarGlyphs::parse(&value)?;
    }
    if let Some(value) = option_value(args, "--activity-bar-width") {
        style.bar_cells = value
            .trim()
            .parse()
            .map_err(|_| format!("--activity-bar-width {value:?} must be a whole number"))?;
    }
    if let Some(value) = option_value(args, "--activity-bar-basis") {
        style.relative_to_largest = match value.trim() {
            "largest" => true,
            "total" => false,
            other => {
                return Err(
                    format!("--activity-bar-basis {other:?} must be largest or total").into(),
                );
            }
        };
    }
    Ok(style)
}

fn github_data(
    config: &GithubStatsConfig,
    path: Option<String>,
) -> Result<GithubData, Box<dyn Error>> {
    match path {
        Some(path) => saved_github_data(&path),
        None => live_github_data(config),
    }
}

fn saved_github_data(path: &str) -> Result<GithubData, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let config = GithubStatsConfig::new("fixture")?;
    Ok(
        <MockGithubClient as github_personal_stats_core::GithubClient>::fetch_user_data(
            &MockGithubClient::success(content),
            &config,
        )?,
    )
}

fn live_github_data(config: &GithubStatsConfig) -> Result<GithubData, Box<dyn Error>> {
    Ok(
        <GithubGraphqlClient as github_personal_stats_core::GithubClient>::fetch_user_data(
            &GithubGraphqlClient::new("https://api.github.com/graphql"),
            config,
        )?,
    )
}

fn replace_section(
    source: &str,
    start: &str,
    end: &str,
    replacement: &str,
) -> Result<String, Box<dyn Error>> {
    let start_index = source
        .find(start)
        .ok_or_else(|| format!("missing section marker: {start}"))?;
    let content_start = start_index + start.len();
    let end_offset = source[content_start..]
        .find(end)
        .ok_or_else(|| format!("missing section marker: {end}"))?;
    let end_index = content_start + end_offset;

    Ok(format!(
        "{}{}\n{}\n{}{}",
        &source[..start_index],
        start,
        replacement,
        end,
        &source[end_index + end.len()..]
    ))
}

fn write_output(path: PathBuf, content: String) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn hide_languages(data: &mut GithubData, hidden_languages: &[String]) {
    if hidden_languages.is_empty() {
        return;
    }
    data.languages.retain(|language| {
        !hidden_languages
            .iter()
            .any(|hidden| hidden.eq_ignore_ascii_case(&language.name))
    });
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find_map(|window| (window[0] == name).then(|| window[1].clone()))
}

fn option_values(args: &[String], name: &str) -> Vec<String> {
    args.windows(2)
        .filter(|window| window[0] == name)
        .map(|window| window[1].clone())
        .collect()
}

fn option_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|arg| arg == name)
}
