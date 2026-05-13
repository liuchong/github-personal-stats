use github_personal_stats_core::{
    CodingActivityEntry, GithubData, GithubGraphqlClient, GithubStatsConfig, MockGithubClient,
    aggregate_card_data, aggregate_coding_activity, parse_output_kind, render_card,
    render_readme_section, workspace_info,
};
use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let command = if args.is_empty() {
        "info".to_owned()
    } else {
        args.remove(0)
    };

    match command.as_str() {
        "info" => println!("{}", workspace_info().to_json()),
        "generate" => generate(args)?,
        "update-readme" => update_readme(args)?,
        command => return Err(format!("unsupported command: {command}").into()),
    }

    Ok(())
}

fn generate(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let card = option_value(&args, "--card").unwrap_or_else(|| "dashboard".to_owned());
    let output = option_value(&args, "--output")
        .unwrap_or_else(|| "profile/github-personal-stats.svg".to_owned());
    let user = option_value(&args, "--user").unwrap_or_else(|| "octo".to_owned());
    let width = option_value(&args, "--width")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1000);
    let height = option_value(&args, "--height")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(420);
    let config = GithubStatsConfig::new(user)?.with_size(width, height)?;
    let data = github_data(&config, option_value(&args, "--fixture"))?;
    let card_data = aggregate_card_data(&data, parse_output_kind(&card)?);
    let rendered = render_card(&card_data, &config);

    write_output(PathBuf::from(output), rendered)?;
    Ok(())
}

fn update_readme(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let target =
        PathBuf::from(option_value(&args, "--target").unwrap_or_else(|| "README.md".to_owned()));
    let section = option_value(&args, "--section").unwrap_or_else(|| "waka".to_owned());
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

fn option_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find_map(|window| (window[0] == name).then(|| window[1].clone()))
}
