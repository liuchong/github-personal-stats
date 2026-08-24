use crate::{
    ContributionDay, GithubData, GithubProfile, GithubStatsError, RepositoryLanguage, UserStats,
};

pub fn parse_github_fixture(input: &str) -> Result<GithubData, GithubStatsError> {
    Ok(GithubData {
        profile: GithubProfile {
            login: required_string(input, "login")?,
            name: optional_string(input, "name"),
            followers: required_number(input, "followers")?,
            public_repositories: required_number(input, "publicRepositories")?,
        },
        stats: UserStats {
            stars: required_number(input, "stars")?,
            commits: required_number(input, "commits")?,
            pull_requests: required_number(input, "pullRequests")?,
            issues: required_number(input, "issues")?,
            reviews: required_number(input, "reviews")?,
            contributed_to: required_number(input, "contributedTo")?,
        },
        languages: parse_languages(input)?,
        contributions: parse_contributions(input)?,
    })
}

/// Writes the shape [`parse_github_fixture`] reads, so a fetch can be saved once
/// and rendered from many times.
///
/// Scalars come before the arrays on purpose: a field is looked up by the first
/// key that matches it, and `name` occurs again inside every language.
pub fn write_github_fixture(data: &GithubData) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"login\": {},\n", string(&data.profile.login)));
    match &data.profile.name {
        Some(name) => out.push_str(&format!("  \"name\": {},\n", string(name))),
        None => out.push_str("  \"name\": null,\n"),
    }
    out.push_str(&format!("  \"followers\": {},\n", data.profile.followers));
    out.push_str(&format!(
        "  \"publicRepositories\": {},\n",
        data.profile.public_repositories
    ));
    out.push_str(&format!("  \"stars\": {},\n", data.stats.stars));
    out.push_str(&format!("  \"commits\": {},\n", data.stats.commits));
    out.push_str(&format!(
        "  \"pullRequests\": {},\n",
        data.stats.pull_requests
    ));
    out.push_str(&format!("  \"issues\": {},\n", data.stats.issues));
    out.push_str(&format!("  \"reviews\": {},\n", data.stats.reviews));
    out.push_str(&format!(
        "  \"contributedTo\": {},\n",
        data.stats.contributed_to
    ));

    out.push_str("  \"languages\": [\n");
    for (index, language) in data.languages.iter().enumerate() {
        out.push_str(&format!(
            "    {{ \"name\": {}, \"size\": {} }}",
            string(&language.name),
            language.size
        ));
        out.push_str(if index + 1 == data.languages.len() {
            "\n"
        } else {
            ",\n"
        });
    }
    out.push_str("  ],\n");

    out.push_str("  \"contributions\": [\n");
    for (index, day) in data.contributions.iter().enumerate() {
        out.push_str(&format!(
            "    {{ \"date\": {}, \"count\": {} }}",
            string(&day.date),
            day.count
        ));
        out.push_str(if index + 1 == data.contributions.len() {
            "\n"
        } else {
            ",\n"
        });
    }
    out.push_str("  ]\n");
    out.push_str("}\n");
    out
}

fn string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if (character as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}

fn parse_languages(input: &str) -> Result<Vec<RepositoryLanguage>, GithubStatsError> {
    array_items(input, "languages")
        .into_iter()
        .map(|item| {
            Ok(RepositoryLanguage {
                name: required_string(item, "name")?,
                size: required_number(item, "size")?,
            })
        })
        .collect()
}

fn parse_contributions(input: &str) -> Result<Vec<ContributionDay>, GithubStatsError> {
    array_items(input, "contributions")
        .into_iter()
        .map(|item| {
            Ok(ContributionDay {
                date: required_string(item, "date")?,
                count: required_number::<u32>(item, "count")?,
            })
        })
        .collect()
}

fn required_string(input: &str, key: &str) -> Result<String, GithubStatsError> {
    optional_string(input, key).ok_or_else(|| GithubStatsError::InvalidResponse {
        message: format!("missing string field {key}"),
    })
}

fn optional_string(input: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\"");
    let start = input.find(&marker)? + marker.len();
    let after_colon = input[start..].find(':')? + start + 1;
    let value = input[after_colon..].trim_start();
    if value.starts_with("null") {
        return None;
    }
    let value = value.strip_prefix('"')?;
    let mut characters = value.chars();
    let mut text = String::new();
    while let Some(character) = characters.next() {
        match character {
            '"' => return Some(text),
            '\\' => text.push(unescape(&mut characters)?),
            character => text.push(character),
        }
    }
    None
}

fn unescape(characters: &mut std::str::Chars<'_>) -> Option<char> {
    match characters.next()? {
        '"' => Some('"'),
        '\\' => Some('\\'),
        '/' => Some('/'),
        'n' => Some('\n'),
        'r' => Some('\r'),
        't' => Some('\t'),
        'b' => Some('\u{8}'),
        'f' => Some('\u{c}'),
        'u' => {
            let digits = characters.by_ref().take(4).collect::<String>();
            char::from_u32(u32::from_str_radix(&digits, 16).ok()?)
        }
        _ => None,
    }
}

fn required_number<T>(input: &str, key: &str) -> Result<T, GithubStatsError>
where
    T: TryFrom<u64>,
{
    let marker = format!("\"{key}\"");
    let start = input
        .find(&marker)
        .ok_or_else(|| GithubStatsError::InvalidResponse {
            message: format!("missing number field {key}"),
        })?
        + marker.len();
    let after_colon =
        input[start..]
            .find(':')
            .ok_or_else(|| GithubStatsError::InvalidResponse {
                message: format!("missing number separator for {key}"),
            })?
            + start
            + 1;
    let digits = input[after_colon..]
        .trim_start()
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    let value = digits
        .parse::<u64>()
        .map_err(|_| GithubStatsError::InvalidResponse {
            message: format!("invalid number field {key}"),
        })?;
    T::try_from(value).map_err(|_| GithubStatsError::InvalidResponse {
        message: format!("number out of range for {key}"),
    })
}

fn array_items<'a>(input: &'a str, key: &str) -> Vec<&'a str> {
    let marker = format!("\"{key}\"");
    let Some(start) = input.find(&marker) else {
        return Vec::new();
    };
    let Some(array_start_offset) = input[start..].find('[') else {
        return Vec::new();
    };
    let array_start = start + array_start_offset + 1;
    let mut depth = 0_u32;
    let mut item_start = None;
    let mut items = Vec::new();

    for (offset, character) in input[array_start..].char_indices() {
        let index = array_start + offset;
        match character {
            '{' => {
                if depth == 0 {
                    item_start = Some(index);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = item_start.take() {
                        items.push(&input[start..=index]);
                    }
                }
            }
            ']' if depth == 0 => break,
            _ => {}
        }
    }

    items
}
