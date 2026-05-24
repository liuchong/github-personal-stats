use github_personal_stats_core::json::parse_github_fixture;

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
