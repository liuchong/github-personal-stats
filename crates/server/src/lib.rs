use github_personal_stats_core::{
    CodingActivityEntry, ContributionDay, GithubData, GithubProfile, GithubStatsConfig, ImageSize,
    OutputKind, RepositoryLanguage, UserStats, aggregate_card_data, aggregate_coding_activity,
    parse_output_kind, render_card, render_readme_section, workspace_info,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

pub fn handle_request(path: &str) -> HttpResponse {
    let (route, query) = split_query(path);

    match route {
        "/health" => text_response(200, "ok"),
        "/info" => json_response(200, workspace_info().to_json()),
        "/api" => render_api(query, OutputKind::Dashboard),
        "/api/dashboard" => render_api(query, OutputKind::Dashboard),
        "/api/stats" => render_api(query, OutputKind::Stats),
        "/api/languages" => render_api(query, OutputKind::Languages),
        "/api/streak" => render_api(query, OutputKind::Streak),
        "/api/wakatime" => render_api(query, OutputKind::Wakatime),
        "/api/status" => render_api(query, OutputKind::Status),
        "/api/wakatime-text" => text_response(200, render_coding_activity_preview()),
        _ => text_response(404, "not found"),
    }
}

pub fn http_bytes(response: HttpResponse) -> Vec<u8> {
    format!(
        "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: public, max-age=300\r\n\r\n{}",
        response.status,
        response.content_type,
        response.body.len(),
        response.body
    )
    .into_bytes()
}

fn render_api(query: &str, fallback: OutputKind) -> HttpResponse {
    let card = query_value(query, "card")
        .and_then(|value| parse_output_kind(&value).ok())
        .unwrap_or(fallback);
    let width = query_value(query, "width")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1000);
    let height = query_value(query, "height")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(420);
    let username = query_value(query, "username").unwrap_or_else(|| "octo".to_owned());
    let size = ImageSize::new(width, height).unwrap_or(ImageSize {
        width: 1000,
        height: 420,
    });
    let config = GithubStatsConfig {
        username,
        token_env: "GITHUB_TOKEN".to_owned(),
        cards: github_personal_stats_core::CardSelection {
            outputs: vec![card],
        },
        size,
        theme: query_value(query, "theme").unwrap_or_else(|| "default".to_owned()),
    };
    let body = render_card(&aggregate_card_data(&sample_github_data(), card), &config);

    HttpResponse {
        status: 200,
        content_type: "image/svg+xml; charset=utf-8",
        body,
    }
}

fn render_coding_activity_preview() -> String {
    let summary = aggregate_coding_activity(
        vec![
            CodingActivityEntry {
                language: "Rust".to_owned(),
                seconds: 7200,
            },
            CodingActivityEntry {
                language: "Shell".to_owned(),
                seconds: 1800,
            },
        ],
        8,
        &[],
        true,
    );
    render_readme_section(&summary, "Coding Activity")
}

fn split_query(path: &str) -> (&str, &str) {
    if let Some(index) = path.find('?') {
        (&path[..index], &path[index + 1..])
    } else {
        (path, "")
    }
}

fn query_value(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then(|| value.replace('+', " "))
    })
}

fn text_response(status: u16, body: impl Into<String>) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "text/plain; charset=utf-8",
        body: body.into(),
    }
}

fn json_response(status: u16, body: String) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "application/json; charset=utf-8",
        body,
    }
}

fn sample_github_data() -> GithubData {
    GithubData {
        profile: GithubProfile {
            login: "octo".to_owned(),
            name: Some("Octo User".to_owned()),
            followers: 42,
            public_repositories: 7,
        },
        stats: UserStats {
            stars: 120,
            commits: 350,
            pull_requests: 21,
            issues: 13,
            reviews: 8,
            contributed_to: 5,
        },
        languages: vec![
            RepositoryLanguage {
                name: "Rust".to_owned(),
                size: 5000,
            },
            RepositoryLanguage {
                name: "TypeScript".to_owned(),
                size: 3000,
            },
            RepositoryLanguage {
                name: "Shell".to_owned(),
                size: 400,
            },
        ],
        contributions: vec![
            ContributionDay {
                date: "2026-05-10".to_owned(),
                count: 1,
            },
            ContributionDay {
                date: "2026-05-11".to_owned(),
                count: 0,
            },
            ContributionDay {
                date: "2026-05-12".to_owned(),
                count: 3,
            },
        ],
    }
}
