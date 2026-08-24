use github_personal_stats_core::json::{parse_github_fixture, write_github_fixture};
use github_personal_stats_core::{
    ContributionDay, GithubData, GithubProfile, RepositoryLanguage, UserStats,
};

#[test]
fn fixture_parser_reports_missing_required_string() {
    let error = parse_github_fixture(
        r#"{
          "name": null,
          "followers": 1,
          "publicRepositories": 2,
          "stars": 3,
          "commits": 4,
          "pullRequests": 5,
          "issues": 6,
          "reviews": 7,
          "contributedTo": 8,
          "languages": [],
          "contributions": []
        }"#,
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "invalid response: missing string field login"
    );
}

#[test]
fn fixture_parser_reports_invalid_and_out_of_range_numbers() {
    let invalid = parse_github_fixture(
        r#"{
          "login": "octo",
          "followers": "many",
          "publicRepositories": 2,
          "stars": 3,
          "commits": 4,
          "pullRequests": 5,
          "issues": 6,
          "reviews": 7,
          "contributedTo": 8,
          "languages": [],
          "contributions": []
        }"#,
    )
    .unwrap_err();
    let out_of_range = parse_github_fixture(
        r#"{
          "login": "octo",
          "followers": 1,
          "publicRepositories": 2,
          "stars": 3,
          "commits": 4,
          "pullRequests": 5,
          "issues": 6,
          "reviews": 7,
          "contributedTo": 8,
          "languages": [],
          "contributions": [{
            "date": "2026-05-24",
            "count": 4294967296
          }]
        }"#,
    )
    .unwrap_err();

    assert_eq!(
        invalid.to_string(),
        "invalid response: invalid number field followers"
    );
    assert_eq!(
        out_of_range.to_string(),
        "invalid response: number out of range for count"
    );
}

#[test]
fn fixture_parser_treats_missing_arrays_as_empty() {
    let data = parse_github_fixture(
        r#"{
          "login": "octo",
          "name": null,
          "followers": 1,
          "publicRepositories": 2,
          "stars": 3,
          "commits": 4,
          "pullRequests": 5,
          "issues": 6,
          "reviews": 7,
          "contributedTo": 8
        }"#,
    )
    .unwrap();

    assert!(data.languages.is_empty());
    assert!(data.contributions.is_empty());
}

#[test]
fn a_saved_fetch_reads_back_as_what_was_saved() {
    let saved = GithubData {
        profile: GithubProfile {
            login: "octo".to_owned(),
            name: Some(r#"Ada "Tab" Lovelace \ 刘"#.to_owned()),
            followers: 12,
            public_repositories: 34,
        },
        stats: UserStats {
            stars: 7_341,
            commits: 687,
            pull_requests: 442,
            issues: 52,
            reviews: 19,
            contributed_to: 8,
        },
        languages: vec![
            RepositoryLanguage {
                name: "C++".to_owned(),
                size: 4_096,
            },
            RepositoryLanguage {
                name: r#"a "quoted" tongue"#.to_owned(),
                size: 1,
            },
        ],
        contributions: vec![
            ContributionDay {
                date: "2026-04-27".to_owned(),
                count: 3,
            },
            ContributionDay {
                date: "2026-04-28".to_owned(),
                count: 0,
            },
        ],
    };

    let read_back = parse_github_fixture(&write_github_fixture(&saved)).unwrap();

    assert_eq!(read_back, saved);
}

#[test]
fn a_missing_display_name_is_not_filled_in_from_a_language() {
    // A field is found by the first key that matches it, and every language
    // carries a name of its own, so the order the two are written in matters.
    let anonymous = GithubData {
        profile: GithubProfile {
            login: "octo".to_owned(),
            name: None,
            followers: 0,
            public_repositories: 0,
        },
        stats: UserStats {
            stars: 0,
            commits: 0,
            pull_requests: 0,
            issues: 0,
            reviews: 0,
            contributed_to: 0,
        },
        languages: vec![RepositoryLanguage {
            name: "Rust".to_owned(),
            size: 10,
        }],
        contributions: Vec::new(),
    };

    let read_back = parse_github_fixture(&write_github_fixture(&anonymous)).unwrap();

    assert_eq!(read_back.profile.name, None);
    assert_eq!(read_back, anonymous);
}

#[test]
fn a_hand_written_example_survives_a_round_trip() {
    let example = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/showcase.json"
    ))
    .unwrap();
    let parsed = parse_github_fixture(&example).unwrap();

    let read_back = parse_github_fixture(&write_github_fixture(&parsed)).unwrap();

    assert_eq!(read_back, parsed);
}
