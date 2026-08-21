use github_personal_stats_core::{
    CodingActivityEntry, DEFAULT_HEAT_THRESHOLD, DEFAULT_LANGUAGE_ROWS, GithubData,
    GithubGraphqlClient, GithubStatsConfig, MAX_LANGUAGE_ROWS, MockGithubClient,
    aggregate_card_data, aggregate_coding_activity, parse_output_kind, render_card,
    render_readme_section, workspace_info,
};
use std::{env, error::Error, fs, path::PathBuf};

const DEFAULT_CARD: &str = "dashboard";
const DEFAULT_OUTPUT: &str = "profile/github-personal-stats.svg";
const DEFAULT_USER: &str = "octo";
const DEFAULT_WIDTH: u32 = 1000;
const DEFAULT_HEIGHT: u32 = 420;
const DEFAULT_THEME: &str = "light";
const DEFAULT_TARGET: &str = "README.md";
const DEFAULT_SECTION: &str = "waka";
const DEFAULT_STAT_ROWS: &str = "stars,commits,prs,issues";
const DEFAULT_STREAK_SIDES: &str = "total,longest";

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
        "generate" => generate(args)?,
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
  generate                Render a card to an SVG file
  update-readme           Rewrite a marked section of a README
  info                    Print workspace information as JSON
  help, --help, -h        Print this message

Generate options:
  --user <login>          Profile to read (default: {DEFAULT_USER})
  --card <kind>           dashboard, stats, languages, streak, wakatime, status
                          (default: {DEFAULT_CARD})
  --output <path>         Where to write the card (default: {DEFAULT_OUTPUT})
  --width <pixels>        Card width (default: {DEFAULT_WIDTH})
  --height <pixels>       Card height (default: {DEFAULT_HEIGHT})
  --theme <name>          light, dark, transparent (default: {DEFAULT_THEME})
  --fixture <path>        Read sanitized fixture JSON instead of the network
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

Update-readme options:
  --target <path>         README to rewrite (default: {DEFAULT_TARGET})
  --section <name>        Marker name to replace (default: {DEFAULT_SECTION})

Environment:
  GITHUB_TOKEN            Token used for live GitHub data

Card aliases top-langs, top-languages, and coding-activity are accepted.
Ring options are illustrated in docs/user-guide.md."
    )
}

fn generate(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let card = option_value(&args, "--card").unwrap_or_else(|| DEFAULT_CARD.to_owned());
    let output = option_value(&args, "--output").unwrap_or_else(|| DEFAULT_OUTPUT.to_owned());
    let user = option_value(&args, "--user").unwrap_or_else(|| DEFAULT_USER.to_owned());
    let width = option_value(&args, "--width")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(DEFAULT_WIDTH);
    let height = option_value(&args, "--height")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(DEFAULT_HEIGHT);
    let mut config = GithubStatsConfig::new(user)?.with_size(width, height)?;
    if let Some(value) = option_value(&args, "--theme") {
        config = config.with_theme(&value)?;
    }
    if option_flag(&args, "--authored-languages") {
        config = config.with_authored_languages();
    }
    config = config.with_author_emails(option_values(&args, "--author-email"));
    config = config.with_hidden_languages(option_values(&args, "--hide-language"));
    if let Some(value) = option_value(&args, "--min-repo-language-share") {
        config = config.with_min_repo_language_share(&value)?;
    }
    if let Some(value) = option_value(&args, "--stat-rows") {
        config = config.with_stat_rows(&value)?;
    }
    if let Some(value) = option_value(&args, "--language-rows") {
        config = config.with_language_rows(&value)?;
    }
    if let Some(value) = option_value(&args, "--streak-sides") {
        config = config.with_streak_sides(&value)?;
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
    let mut data = github_data(&config, option_value(&args, "--fixture"))?;
    hide_languages(&mut data, &config.hidden_languages);
    let card_data = aggregate_card_data(&data, parse_output_kind(&card)?, &config.heat_ring);
    let rendered = render_card(&card_data, &config);

    write_output(PathBuf::from(output), rendered)?;
    Ok(())
}

fn update_readme(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let target =
        PathBuf::from(option_value(&args, "--target").unwrap_or_else(|| DEFAULT_TARGET.to_owned()));
    let section = option_value(&args, "--section").unwrap_or_else(|| DEFAULT_SECTION.to_owned());
    let start = format!("<!--START_SECTION:{section}-->");
    let end = format!("<!--END_SECTION:{section}-->");
    let source = fs::read_to_string(&target)?;
    let summary = aggregate_coding_activity(sample_coding_activity(), 8, &[], true);
    let replacement = render_readme_section(&summary, "Coding Activity");
    let updated = replace_section(&source, &start, &end, &replacement)?;

    fs::write(target, updated)?;
    Ok(())
}

fn github_data(
    config: &GithubStatsConfig,
    path: Option<String>,
) -> Result<GithubData, Box<dyn Error>> {
    if let Some(path) = path {
        let content = fs::read_to_string(path)?;
        let config = GithubStatsConfig::new("fixture")?;
        return Ok(
            <MockGithubClient as github_personal_stats_core::GithubClient>::fetch_user_data(
                &MockGithubClient::success(content),
                &config,
            )?,
        );
    }

    Ok(
        <GithubGraphqlClient as github_personal_stats_core::GithubClient>::fetch_user_data(
            &GithubGraphqlClient::new("https://api.github.com/graphql"),
            config,
        )?,
    )
}

fn sample_coding_activity() -> Vec<CodingActivityEntry> {
    vec![
        CodingActivityEntry {
            language: "Rust".to_owned(),
            seconds: 7200,
        },
        CodingActivityEntry {
            language: "Shell".to_owned(),
            seconds: 1800,
        },
    ]
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
