use github_personal_stats_core::{
    GithubClient, GithubGraphqlClient, GithubStatsConfig, LanguageScope, MockGithubClient,
    RemoteErrorKind,
};

const FIXTURE: &str = include_str!("fixtures/github_user_data.json");

#[test]
fn config_defaults_to_dashboard_and_standard_token_env() {
    let config = GithubStatsConfig::new("octo").unwrap();

    assert_eq!(config.username, "octo");
    assert_eq!(config.token_env, "GITHUB_TOKEN");
    assert_eq!(config.size.width, 1000);
    assert_eq!(config.size.height, 420);
    assert_eq!(config.language_scope, LanguageScope::Owned);
    assert!(config.author_emails.is_empty());
    assert!(config.hidden_languages.is_empty());
    assert_eq!(config.min_repo_language_share_basis_points, 0);
}

#[test]
fn config_can_enable_authored_language_scope() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_authored_languages();

    assert_eq!(config.language_scope, LanguageScope::Authored);
}

#[test]
fn config_trims_author_email_supplements() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_author_emails(vec![
            " first@example.com ".to_owned(),
            "second@example.com, third@example.com ".to_owned(),
        ]);

    assert_eq!(
        config.author_emails,
        vec![
            "first@example.com",
            "second@example.com",
            "third@example.com"
        ]
    );
}

#[test]
fn config_trims_hidden_languages() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_hidden_languages(vec![
            " Ruby ".to_owned(),
            "CSS, HTML ".to_owned(),
            "".to_owned(),
        ]);

    assert_eq!(config.hidden_languages, vec!["Ruby", "CSS", "HTML"]);
}

#[test]
fn config_parses_min_repo_language_share_percentage() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_min_repo_language_share("2.5")
        .unwrap();

    assert_eq!(config.min_repo_language_share_basis_points, 250);
}

#[test]
fn config_rejects_invalid_min_repo_language_share() {
    let error = GithubStatsConfig::new("octo")
        .unwrap()
        .with_min_repo_language_share("101")
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "invalid config for min_repo_language_share: must be a percentage between 0 and 100"
    );
}

#[test]
fn config_rejects_empty_username() {
    let error = GithubStatsConfig::new(" ").unwrap_err();

    assert_eq!(
        error.to_string(),
        "invalid config for username: username is required"
    );
}

#[test]
fn card_selection_accepts_multiple_outputs() {
    let config = GithubStatsConfig::new("octo")
        .unwrap()
        .with_cards("stats,languages,streak")
        .unwrap();

    assert_eq!(config.cards.outputs.len(), 3);
}

#[test]
fn image_size_rejects_zero_dimension() {
    let error = GithubStatsConfig::new("octo")
        .unwrap()
        .with_size(0, 420)
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "invalid config for size: width and height must be positive"
    );
}

#[test]
fn graphql_request_contains_endpoint_token_env_and_username() {
    let config = GithubStatsConfig::new("octo").unwrap();
    let client = GithubGraphqlClient::new("https://api.github.com/graphql");
    let request = client.build_user_data_request(&config);

    assert_eq!(request.endpoint, "https://api.github.com/graphql");
    assert_eq!(request.token_env, "GITHUB_TOKEN");
    assert!(request.body.contains("pullRequests"));
    assert!(request.body.contains("issues"));
    assert!(request.body.contains("contributionYears"));
    assert!(request.body.contains("nameWithOwner"));
    assert!(request.body.contains("totalSize"));
}

#[test]
fn mock_client_parses_sanitized_fixture() {
    let config = GithubStatsConfig::new("octo").unwrap();
    let client = MockGithubClient::success(FIXTURE);
    let data = client.fetch_user_data(&config).unwrap();

    assert_eq!(data.profile.login, "octo");
    assert_eq!(data.profile.followers, 42);
    assert_eq!(data.stats.stars, 120);
    assert_eq!(data.languages[0].name, "Rust");
    assert_eq!(data.contributions[2].count, 3);
}

#[test]
fn mock_client_preserves_remote_error_classification() {
    let config = GithubStatsConfig::new("octo").unwrap();
    let client = MockGithubClient::failure(RemoteErrorKind::RateLimit, "rate limit exceeded");
    let error = client.fetch_user_data(&config).unwrap_err();

    assert_eq!(
        error.to_string(),
        "remote error RateLimit: rate limit exceeded"
    );
}

#[test]
fn graphql_client_requires_token_env_for_live_fetch() {
    let mut config = GithubStatsConfig::new("octo").unwrap();
    config.token_env = "GITHUB_PERSONAL_STATS_TEST_MISSING_TOKEN".to_owned();
    let client = GithubGraphqlClient::new("https://api.github.com/graphql");
    let error = client.fetch_user_data(&config).unwrap_err();

    assert_eq!(
        error.to_string(),
        "remote error Authentication: missing token environment variable GITHUB_PERSONAL_STATS_TEST_MISSING_TOKEN"
    );
}
