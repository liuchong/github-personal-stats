//! What a profile is, and how what GitHub sends back becomes one.
//!
//! Reading the network lives in `remote`, which cannot be tested locally. This
//! file holds the parts that can: the request that would be sent, the shapes of
//! the replies, and every decision made about them once they arrive.

use crate::{
    ContributionDay, GithubData, GithubProfile, GithubStatsConfig, GithubStatsError, LanguageScope,
    RemoteErrorKind, RepositoryLanguage, UserStats, json::parse_github_fixture,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubGraphqlRequest {
    pub endpoint: String,
    pub token_env: String,
    pub body: String,
}

pub trait GithubClient {
    fn fetch_user_data(&self, config: &GithubStatsConfig) -> Result<GithubData, GithubStatsError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockGithubClient {
    response: Result<String, GithubStatsError>,
}

impl MockGithubClient {
    pub fn success(response: impl Into<String>) -> Self {
        Self {
            response: Ok(response.into()),
        }
    }

    pub fn failure(kind: RemoteErrorKind, message: impl Into<String>) -> Self {
        Self {
            response: Err(GithubStatsError::Remote {
                kind,
                message: message.into(),
            }),
        }
    }
}

impl GithubClient for MockGithubClient {
    fn fetch_user_data(&self, _config: &GithubStatsConfig) -> Result<GithubData, GithubStatsError> {
        parse_github_fixture(self.response.clone()?.as_str())
    }
}

pub(crate) fn aggregate_repository_languages(
    config: &GithubStatsConfig,
    repositories: &[RepositoryNode],
    authored_repository_ids: Option<&BTreeSet<String>>,
) -> Vec<RepositoryLanguage> {
    let mut languages = BTreeMap::<String, u64>::new();
    for repository in repositories {
        if repository.is_fork {
            continue;
        }
        if let LanguageScope::Authored = config.language_scope
            && !authored_repository_ids
                .map(|repository_ids| repository_ids.contains(&repository.id))
                .unwrap_or(false)
        {
            continue;
        }
        for edge in &repository.languages.edges {
            if config.min_repo_language_share_basis_points > 0
                && repository.languages.total_size > 0
                && (u128::from(edge.size) * 10_000)
                    < (u128::from(repository.languages.total_size)
                        * u128::from(config.min_repo_language_share_basis_points))
            {
                continue;
            }
            *languages.entry(edge.node.name.clone()).or_default() += edge.size;
        }
    }
    languages
        .into_iter()
        .map(|(name, size)| RepositoryLanguage { name, size })
        .collect()
}

pub(crate) fn assemble_github_data(
    config: &GithubStatsConfig,
    user: ProfileUser,
    repositories: RepositoryConnection,
    authored_repository_ids: Option<&BTreeSet<String>>,
    contributions: BTreeMap<String, u32>,
) -> GithubData {
    let languages =
        aggregate_repository_languages(config, &repositories.nodes, authored_repository_ids);

    GithubData {
        profile: GithubProfile {
            login: user.login,
            name: user.name,
            followers: user.followers.total_count,
            public_repositories: repositories.total_count,
        },
        stats: UserStats {
            stars: repositories
                .nodes
                .iter()
                .filter(|repository| !repository.is_fork)
                .map(|repository| repository.stargazer_count)
                .sum(),
            commits: user.contributions_collection.total_commit_contributions,
            pull_requests: user.pull_requests.total_count,
            issues: user.issues.total_count,
            reviews: user
                .contributions_collection
                .total_pull_request_review_contributions,
            contributed_to: user.repositories_contributed_to.total_count,
        },
        languages,
        contributions: contributions
            .into_iter()
            .map(|(date, count)| ContributionDay { date, count })
            .collect(),
    }
}

pub(crate) fn http_error_kind(status: u16, body: &str) -> RemoteErrorKind {
    if status == 401 || body.contains("Bad credentials") {
        RemoteErrorKind::Authentication
    } else if status == 403 && body.to_ascii_lowercase().contains("rate limit") {
        RemoteErrorKind::RateLimit
    } else if status == 403 {
        RemoteErrorKind::Permission
    } else if status == 404 || status == 409 {
        RemoteErrorKind::NotFound
    } else {
        RemoteErrorKind::UpstreamUnavailable
    }
}

pub(crate) fn ensure_success_body(body: &str) -> Result<(), GithubStatsError> {
    let Some((status, response_body)) = body.split_once('\n') else {
        return Ok(());
    };
    let Ok(status) = status.parse::<u16>() else {
        return Ok(());
    };
    Err(GithubStatsError::Remote {
        kind: http_error_kind(status, response_body),
        message: response_body.to_owned(),
    })
}

pub(crate) fn retryable_body(error: &GithubStatsError) -> bool {
    matches!(
        error,
        GithubStatsError::Remote {
            kind: RemoteErrorKind::UpstreamUnavailable,
            ..
        }
    )
}

pub(crate) async fn retry_delay(attempt: usize) {
    tokio::time::sleep(std::time::Duration::from_millis(300 * (attempt as u64 + 1))).await;
}

pub(crate) fn repository_commits_url(name_with_owner: &str, author: &str) -> String {
    let path = name_with_owner
        .split('/')
        .map(percent_encode_component)
        .collect::<Vec<_>>()
        .join("/");
    format!(
        "https://api.github.com/repos/{path}/commits?author={}&per_page=1",
        percent_encode_component(author)
    )
}

fn percent_encode_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

pub(crate) fn graphql_error(errors: Vec<GraphqlError>) -> GithubStatsError {
    let message = errors
        .first()
        .map(|error| error.message.clone())
        .unwrap_or_else(|| "GraphQL error".to_owned());
    let kind = errors
        .first()
        .and_then(|error| error.extensions.as_ref())
        .and_then(|extensions| extensions.code.as_deref())
        .map(|code| match code {
            "NOT_FOUND" => RemoteErrorKind::NotFound,
            "RATE_LIMITED" => RemoteErrorKind::RateLimit,
            "FORBIDDEN" => RemoteErrorKind::Permission,
            _ => RemoteErrorKind::InvalidResponse,
        })
        .unwrap_or_else(|| {
            if message.to_ascii_lowercase().contains("rate limit") {
                RemoteErrorKind::RateLimit
            } else {
                RemoteErrorKind::InvalidResponse
            }
        });
    GithubStatsError::Remote { kind, message }
}

#[derive(Debug, Serialize)]
pub(crate) struct GraphqlRequest<'a, V> {
    pub(crate) query: &'a str,
    pub(crate) variables: V,
}

#[derive(Debug, Serialize)]
pub(crate) struct LoginVariables<'a> {
    pub(crate) login: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct CalendarVariables<'a> {
    pub(crate) login: &'a str,
    pub(crate) from: String,
    pub(crate) to: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthoredRepositoriesVariables<'a> {
    pub(crate) login: &'a str,
    pub(crate) after: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub(crate) struct OwnedRepositoriesVariables<'a> {
    pub(crate) login: &'a str,
    pub(crate) after: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GraphqlResponse<T> {
    pub(crate) data: Option<T>,
    pub(crate) errors: Option<Vec<GraphqlError>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GraphqlError {
    pub(crate) message: String,
    pub(crate) extensions: Option<GraphqlErrorExtensions>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GraphqlErrorExtensions {
    pub(crate) code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProfileData {
    pub(crate) user: Option<ProfileUser>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OwnedRepositoriesData {
    pub(crate) user: Option<OwnedRepositoriesUser>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OwnedRepositoriesUser {
    pub(crate) repositories: RepositoryConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfileUser {
    pub(crate) login: String,
    pub(crate) name: Option<String>,
    pub(crate) followers: TotalCount,
    pub(crate) repositories: RepositoryConnection,
    pub(crate) pull_requests: TotalCount,
    pub(crate) issues: TotalCount,
    pub(crate) repositories_contributed_to: TotalCount,
    pub(crate) contributions_collection: ContributionsCollection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContributionsCollection {
    pub(crate) contribution_years: Vec<i32>,
    pub(crate) total_commit_contributions: u64,
    pub(crate) total_pull_request_review_contributions: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TotalCount {
    pub(crate) total_count: u64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryConnection {
    pub(crate) total_count: u64,
    pub(crate) nodes: Vec<RepositoryNode>,
    #[serde(default)]
    pub(crate) page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryNode {
    pub(crate) id: String,
    pub(crate) name_with_owner: String,
    pub(crate) is_fork: bool,
    pub(crate) stargazer_count: u64,
    pub(crate) languages: LanguageConnection,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthoredRepositoriesData {
    pub(crate) user: Option<AuthoredRepositoriesUser>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthoredRepositoriesUser {
    pub(crate) repositories_contributed_to: AuthoredRepositoryConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthoredRepositoryConnection {
    pub(crate) nodes: Vec<AuthoredRepositoryNode>,
    #[serde(default)]
    pub(crate) page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthoredRepositoryNode {
    pub(crate) id: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageInfo {
    pub(crate) has_next_page: bool,
    pub(crate) end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LanguageConnection {
    #[serde(rename = "totalSize")]
    pub(crate) total_size: u64,
    pub(crate) edges: Vec<LanguageEdge>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LanguageEdge {
    pub(crate) size: u64,
    pub(crate) node: LanguageNode,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LanguageNode {
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CalendarData {
    pub(crate) user: Option<CalendarUser>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalendarUser {
    pub(crate) contributions_collection: CalendarContributionsCollection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalendarContributionsCollection {
    pub(crate) contribution_calendar: ContributionCalendar,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ContributionCalendar {
    pub(crate) weeks: Vec<ContributionWeek>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContributionWeek {
    pub(crate) contribution_days: Vec<ContributionDayNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContributionDayNode {
    pub(crate) date: String,
    pub(crate) contribution_count: u32,
}

pub(crate) const PROFILE_QUERY: &str = r#"
query GitHubPersonalStatsProfile($login: String!) {
  user(login: $login) {
    login
    name
    followers { totalCount }
    repositories(first: 100, ownerAffiliations: OWNER, orderBy: {field: STARGAZERS, direction: DESC}) {
      totalCount
      pageInfo { hasNextPage endCursor }
      nodes {
        id
        nameWithOwner
        isFork
        stargazerCount
        languages(first: 10, orderBy: {field: SIZE, direction: DESC}) {
          totalSize
          edges { size node { name } }
        }
      }
    }
    pullRequests(first: 1) { totalCount }
    issues(first: 1) { totalCount }
    repositoriesContributedTo(first: 1, contributionTypes: [COMMIT, ISSUE, PULL_REQUEST, REPOSITORY]) { totalCount }
    contributionsCollection {
      contributionYears
      totalCommitContributions
      totalPullRequestReviewContributions
    }
  }
}
"#;

pub(crate) const OWNED_REPOSITORIES_QUERY: &str = r#"
query GitHubPersonalStatsOwnedRepositories($login: String!, $after: String) {
  user(login: $login) {
    repositories(first: 100, after: $after, ownerAffiliations: OWNER, orderBy: {field: STARGAZERS, direction: DESC}) {
      totalCount
      pageInfo { hasNextPage endCursor }
      nodes {
        id
        nameWithOwner
        isFork
        stargazerCount
        languages(first: 10, orderBy: {field: SIZE, direction: DESC}) {
          totalSize
          edges { size node { name } }
        }
      }
    }
  }
}
"#;

pub(crate) const AUTHORED_REPOSITORIES_QUERY: &str = r#"
query GitHubPersonalStatsAuthoredRepositories($login: String!, $after: String) {
  user(login: $login) {
    repositoriesContributedTo(first: 100, after: $after, contributionTypes: COMMIT, includeUserRepositories: true, orderBy: {field: STARGAZERS, direction: DESC}) {
      nodes { id }
      pageInfo { hasNextPage endCursor }
    }
  }
}
"#;

pub(crate) const CALENDAR_QUERY: &str = r#"
query GitHubPersonalStatsCalendar($login: String!, $from: DateTime!, $to: DateTime!) {
  user(login: $login) {
    contributionsCollection(from: $from, to: $to) {
      contributionCalendar {
        weeks { contributionDays { date contributionCount } }
      }
    }
  }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_language_scope_keeps_only_owned_repositories_with_commit_contributions() {
        let config = GithubStatsConfig::new("octo")
            .unwrap()
            .with_authored_languages();
        let user = ProfileUser {
            login: "octo".to_owned(),
            name: None,
            followers: TotalCount { total_count: 0 },
            repositories: RepositoryConnection {
                total_count: 2,
                page_info: PageInfo {
                    has_next_page: false,
                    end_cursor: None,
                },
                nodes: vec![
                    repository("owned-authored", false, "Rust", 500),
                    repository("owned-not-authored", false, "Ruby", 700),
                    repository("owned-fork-authored", true, "Shell", 900),
                ],
            },
            pull_requests: TotalCount { total_count: 0 },
            issues: TotalCount { total_count: 0 },
            repositories_contributed_to: TotalCount { total_count: 0 },
            contributions_collection: ContributionsCollection {
                contribution_years: vec![],
                total_commit_contributions: 0,
                total_pull_request_review_contributions: 0,
            },
        };
        let authored_repository_ids = BTreeSet::from([
            "owned-authored".to_owned(),
            "external-authored".to_owned(),
            "owned-fork-authored".to_owned(),
        ]);

        let languages = aggregate_repository_languages(
            &config,
            &user.repositories.nodes,
            Some(&authored_repository_ids),
        );

        assert_eq!(
            languages,
            vec![RepositoryLanguage {
                name: "Rust".to_owned(),
                size: 500,
            }]
        );
    }

    #[test]
    fn owned_language_scope_keeps_all_owned_non_fork_repositories() {
        let config = GithubStatsConfig::new("octo").unwrap();
        let user = ProfileUser {
            login: "octo".to_owned(),
            name: None,
            followers: TotalCount { total_count: 0 },
            repositories: RepositoryConnection {
                total_count: 2,
                page_info: PageInfo {
                    has_next_page: false,
                    end_cursor: None,
                },
                nodes: vec![
                    repository("owned-authored", false, "Rust", 500),
                    repository("owned-not-authored", false, "Ruby", 700),
                    repository("owned-fork-authored", true, "Shell", 900),
                ],
            },
            pull_requests: TotalCount { total_count: 0 },
            issues: TotalCount { total_count: 0 },
            repositories_contributed_to: TotalCount { total_count: 0 },
            contributions_collection: ContributionsCollection {
                contribution_years: vec![],
                total_commit_contributions: 0,
                total_pull_request_review_contributions: 0,
            },
        };

        let languages = aggregate_repository_languages(&config, &user.repositories.nodes, None);

        assert_eq!(
            languages,
            vec![
                RepositoryLanguage {
                    name: "Ruby".to_owned(),
                    size: 700,
                },
                RepositoryLanguage {
                    name: "Rust".to_owned(),
                    size: 500,
                },
            ]
        );
    }

    #[test]
    fn min_repo_language_share_filters_small_per_repository_languages() {
        let config = GithubStatsConfig::new("octo")
            .unwrap()
            .with_min_repo_language_share("5")
            .unwrap();
        let mut repository = repository("mixed", false, "Rust", 950);
        repository.languages.total_size = 1_000;
        repository.languages.edges.push(LanguageEdge {
            size: 49,
            node: LanguageNode {
                name: "Python".to_owned(),
            },
        });

        let languages = aggregate_repository_languages(&config, &[repository], None);

        assert_eq!(
            languages,
            vec![RepositoryLanguage {
                name: "Rust".to_owned(),
                size: 950,
            }]
        );
    }

    #[test]
    fn authored_language_scope_without_authored_ids_returns_empty_languages() {
        let config = GithubStatsConfig::new("octo")
            .unwrap()
            .with_authored_languages();
        let repositories = vec![repository("owned-authored", false, "Rust", 500)];

        let languages = aggregate_repository_languages(&config, &repositories, None);

        assert!(languages.is_empty());
    }

    #[test]
    fn min_repo_language_share_does_not_filter_when_repository_total_is_zero() {
        let config = GithubStatsConfig::new("octo")
            .unwrap()
            .with_min_repo_language_share("5")
            .unwrap();
        let mut repository = repository("empty-total", false, "Rust", 10);
        repository.languages.total_size = 0;

        let languages = aggregate_repository_languages(&config, &[repository], None);

        assert_eq!(
            languages,
            vec![RepositoryLanguage {
                name: "Rust".to_owned(),
                size: 10,
            }]
        );
    }

    #[test]
    fn assembled_live_data_preserves_profile_stats_languages_and_contributions() {
        let config = GithubStatsConfig::new("octo")
            .unwrap()
            .with_authored_languages();
        let user = ProfileUser {
            login: "octo".to_owned(),
            name: Some("Octo Cat".to_owned()),
            followers: TotalCount { total_count: 42 },
            repositories: RepositoryConnection::default(),
            pull_requests: TotalCount { total_count: 7 },
            issues: TotalCount { total_count: 9 },
            repositories_contributed_to: TotalCount { total_count: 11 },
            contributions_collection: ContributionsCollection {
                contribution_years: vec![2025],
                total_commit_contributions: 123,
                total_pull_request_review_contributions: 5,
            },
        };
        let mut rust_repository = repository("rust", false, "Rust", 500);
        rust_repository.stargazer_count = 100;
        let mut ruby_repository = repository("ruby", false, "Ruby", 300);
        ruby_repository.stargazer_count = 50;
        let mut fork_repository = repository("fork", true, "Shell", 900);
        fork_repository.stargazer_count = 1_000;
        let repositories = RepositoryConnection {
            total_count: 3,
            nodes: vec![rust_repository, ruby_repository, fork_repository],
            page_info: PageInfo::default(),
        };
        let authored_repository_ids = BTreeSet::from(["rust".to_owned()]);
        let contributions =
            BTreeMap::from([("2025-01-02".to_owned(), 2), ("2025-01-01".to_owned(), 1)]);

        let data = assemble_github_data(
            &config,
            user,
            repositories,
            Some(&authored_repository_ids),
            contributions,
        );

        assert_eq!(data.profile.login, "octo");
        assert_eq!(data.profile.name.as_deref(), Some("Octo Cat"));
        assert_eq!(data.profile.followers, 42);
        assert_eq!(data.profile.public_repositories, 3);
        assert_eq!(data.stats.stars, 150);
        assert_eq!(data.stats.commits, 123);
        assert_eq!(data.stats.pull_requests, 7);
        assert_eq!(data.stats.issues, 9);
        assert_eq!(data.stats.reviews, 5);
        assert_eq!(data.stats.contributed_to, 11);
        assert_eq!(
            data.languages,
            vec![RepositoryLanguage {
                name: "Rust".to_owned(),
                size: 500,
            }]
        );
        assert_eq!(
            data.contributions,
            vec![
                ContributionDay {
                    date: "2025-01-01".to_owned(),
                    count: 1,
                },
                ContributionDay {
                    date: "2025-01-02".to_owned(),
                    count: 2,
                },
            ]
        );
    }

    #[test]
    fn http_error_kind_classifies_status_and_response_body() {
        assert_eq!(
            http_error_kind(401, "whatever"),
            RemoteErrorKind::Authentication
        );
        assert_eq!(
            http_error_kind(500, "Bad credentials"),
            RemoteErrorKind::Authentication
        );
        assert_eq!(
            http_error_kind(403, "API rate limit exceeded"),
            RemoteErrorKind::RateLimit
        );
        assert_eq!(
            http_error_kind(403, "access denied"),
            RemoteErrorKind::Permission
        );
        assert_eq!(http_error_kind(404, "not found"), RemoteErrorKind::NotFound);
        assert_eq!(http_error_kind(409, "gone"), RemoteErrorKind::NotFound);
        assert_eq!(
            http_error_kind(502, "server error"),
            RemoteErrorKind::UpstreamUnavailable
        );
    }

    #[test]
    fn ensure_success_body_accepts_non_status_payloads() {
        assert!(ensure_success_body("{\"ok\":true}").is_ok());
        assert!(ensure_success_body("abc\nxyz").is_ok());
    }

    #[test]
    fn ensure_success_body_maps_http_error_payload() {
        let error = ensure_success_body("403\nAPI rate limit exceeded").unwrap_err();

        assert_eq!(
            error.to_string(),
            "remote error RateLimit: API rate limit exceeded"
        );
    }

    #[test]
    fn retryable_body_only_retries_upstream_unavailable() {
        let upstream = GithubStatsError::Remote {
            kind: RemoteErrorKind::UpstreamUnavailable,
            message: "temporary network error".to_owned(),
        };
        let auth = GithubStatsError::Remote {
            kind: RemoteErrorKind::Authentication,
            message: "bad token".to_owned(),
        };

        assert!(retryable_body(&upstream));
        assert!(!retryable_body(&auth));
    }

    #[test]
    fn percent_encode_component_escapes_reserved_characters() {
        assert_eq!(
            percent_encode_component("name/with+space@example.com"),
            "name%2Fwith%2Bspace%40example.com"
        );
    }

    #[test]
    fn graphql_error_maps_extension_codes_and_fallbacks() {
        let forbidden = graphql_error(vec![GraphqlError {
            message: "forbidden".to_owned(),
            extensions: Some(GraphqlErrorExtensions {
                code: Some("FORBIDDEN".to_owned()),
            }),
        }]);
        let rate_limited = graphql_error(vec![GraphqlError {
            message: "API rate limit exceeded".to_owned(),
            extensions: None,
        }]);
        let defaulted = graphql_error(vec![]);

        assert_eq!(forbidden.to_string(), "remote error Permission: forbidden");
        assert_eq!(
            rate_limited.to_string(),
            "remote error RateLimit: API rate limit exceeded"
        );
        assert_eq!(
            defaulted.to_string(),
            "remote error InvalidResponse: GraphQL error"
        );
    }

    #[test]
    fn graphql_error_maps_known_extension_codes() {
        let not_found = graphql_error(vec![GraphqlError {
            message: "missing".to_owned(),
            extensions: Some(GraphqlErrorExtensions {
                code: Some("NOT_FOUND".to_owned()),
            }),
        }]);
        let rate_limited = graphql_error(vec![GraphqlError {
            message: "limited".to_owned(),
            extensions: Some(GraphqlErrorExtensions {
                code: Some("RATE_LIMITED".to_owned()),
            }),
        }]);
        let unknown = graphql_error(vec![GraphqlError {
            message: "unknown".to_owned(),
            extensions: Some(GraphqlErrorExtensions {
                code: Some("OTHER".to_owned()),
            }),
        }]);

        assert_eq!(not_found.to_string(), "remote error NotFound: missing");
        assert_eq!(rate_limited.to_string(), "remote error RateLimit: limited");
        assert_eq!(unknown.to_string(), "remote error InvalidResponse: unknown");
    }

    #[test]
    fn deserializes_live_graphql_response_shapes() {
        let profile: GraphqlResponse<ProfileData> = serde_json::from_str(
            r#"{
              "data": {
                "user": {
                  "login": "octo",
                  "name": null,
                  "followers": { "totalCount": 2 },
                  "repositories": {
                    "totalCount": 1,
                    "nodes": [{
                      "id": "repo1",
                      "nameWithOwner": "octo/repo1",
                      "isFork": false,
                      "stargazerCount": 3,
                      "languages": {
                        "totalSize": 4,
                        "edges": [{ "size": 4, "node": { "name": "Rust" } }]
                      }
                    }]
                  },
                  "pullRequests": { "totalCount": 5 },
                  "issues": { "totalCount": 6 },
                  "repositoriesContributedTo": { "totalCount": 7 },
                  "contributionsCollection": {
                    "contributionYears": [2026],
                    "totalCommitContributions": 8,
                    "totalPullRequestReviewContributions": 9
                  }
                }
              }
            }"#,
        )
        .unwrap();
        let user = profile.data.unwrap().user.unwrap();
        assert_eq!(user.login, "octo");
        assert_eq!(
            user.repositories.nodes[0].languages.edges[0].node.name,
            "Rust"
        );
        assert!(!user.repositories.page_info.has_next_page);

        let mut calendar: GraphqlResponse<CalendarData> = serde_json::from_str(
            r#"{
              "data": {
                "user": {
                  "contributionsCollection": {
                    "contributionCalendar": {
                      "weeks": [{
                        "contributionDays": [{
                          "date": "2026-05-24",
                          "contributionCount": 4
                        }]
                      }]
                    }
                  }
                }
              }
            }"#,
        )
        .unwrap();
        let days = calendar
            .data
            .as_mut()
            .unwrap()
            .user
            .as_mut()
            .unwrap()
            .contributions_collection
            .contribution_calendar
            .weeks
            .remove(0)
            .contribution_days;
        assert_eq!(days[0].date, "2026-05-24");
        assert_eq!(days[0].contribution_count, 4);
    }

    #[test]
    fn deserializes_paginated_repository_response_shapes() {
        let owned: GraphqlResponse<OwnedRepositoriesData> = serde_json::from_str(
            r#"{
              "data": {
                "user": {
                  "repositories": {
                    "totalCount": 1,
                    "pageInfo": { "hasNextPage": true, "endCursor": "next" },
                    "nodes": [{
                      "id": "repo2",
                      "nameWithOwner": "octo/repo2",
                      "isFork": true,
                      "stargazerCount": 10,
                      "languages": {
                        "totalSize": 20,
                        "edges": [{ "size": 20, "node": { "name": "TypeScript" } }]
                      }
                    }]
                  }
                }
              }
            }"#,
        )
        .unwrap();
        let repositories = owned.data.unwrap().user.unwrap().repositories;
        assert!(repositories.page_info.has_next_page);
        assert_eq!(repositories.page_info.end_cursor.as_deref(), Some("next"));
        assert!(repositories.nodes[0].is_fork);

        let authored: GraphqlResponse<AuthoredRepositoriesData> = serde_json::from_str(
            r#"{
              "data": {
                "user": {
                  "repositoriesContributedTo": {
                    "nodes": [{ "id": "repo2" }],
                    "pageInfo": { "hasNextPage": false, "endCursor": null }
                  }
                }
              }
            }"#,
        )
        .unwrap();
        let repositories = authored
            .data
            .unwrap()
            .user
            .unwrap()
            .repositories_contributed_to;
        assert_eq!(repositories.nodes[0].id, "repo2");
        assert!(!repositories.page_info.has_next_page);
        assert_eq!(repositories.page_info.end_cursor, None);
    }

    fn repository(id: &str, is_fork: bool, language: &str, size: u64) -> RepositoryNode {
        RepositoryNode {
            id: id.to_owned(),
            name_with_owner: format!("octo/{id}"),
            is_fork,
            stargazer_count: 0,
            languages: LanguageConnection {
                total_size: size,
                edges: vec![LanguageEdge {
                    size,
                    node: LanguageNode {
                        name: language.to_owned(),
                    },
                }],
            },
        }
    }

    #[test]
    fn repository_commits_url_encodes_author_query() {
        assert_eq!(
            repository_commits_url("liuchong/uluru-push", "liuchong@xindong.com"),
            "https://api.github.com/repos/liuchong/uluru-push/commits?author=liuchong%40xindong.com&per_page=1"
        );
    }
}
