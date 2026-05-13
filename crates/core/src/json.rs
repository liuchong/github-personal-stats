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
    let end = value.find('"')?;
    Some(value[..end].to_owned())
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
