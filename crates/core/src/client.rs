use crate::{
    ContributionDay, GithubData, GithubProfile, GithubStatsConfig, GithubStatsError, LanguageScope,
    RemoteErrorKind, RepositoryLanguage, UserStats, json::parse_github_fixture,
};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{
    Method, Request,
    header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;

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
pub struct GithubGraphqlClient {
    endpoint: String,
}

impl GithubGraphqlClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    pub fn build_user_data_request(&self, config: &GithubStatsConfig) -> GithubGraphqlRequest {
        GithubGraphqlRequest {
            endpoint: self.endpoint.clone(),
            token_env: config.token_env.clone(),
            body: PROFILE_QUERY.to_owned(),
        }
    }

    fn token(&self, config: &GithubStatsConfig) -> Result<String, GithubStatsError> {
        env::var(&config.token_env).map_err(|_| GithubStatsError::Remote {
            kind: RemoteErrorKind::Authentication,
            message: format!("missing token environment variable {}", config.token_env),
        })
    }

    async fn post<T: for<'de> Deserialize<'de>, V: Serialize>(
        &self,
        token: &str,
        query: &str,
        variables: V,
    ) -> Result<T, GithubStatsError> {
        let body = Bytes::from(
            serde_json::to_vec(&GraphqlRequest { query, variables }).map_err(|error| {
                GithubStatsError::InvalidResponse {
                    message: error.to_string(),
                }
            })?,
        );
        let mut last_error = None;
        for attempt in 0..3 {
            let request = Request::builder()
                .method(Method::POST)
                .uri(&self.endpoint)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header(CONTENT_TYPE, "application/json")
                .header(USER_AGENT, "github-personal-stats")
                .body(Full::new(body.clone()))
                .map_err(|error| GithubStatsError::InvalidResponse {
                    message: error.to_string(),
                })?;
            let Some(body) = self.request_body(request).await? else {
                retry_delay(attempt).await;
                continue;
            };
            if let Err(error) = ensure_success_body(&body) {
                if retryable_body(&error) && attempt < 2 {
                    last_error = Some(error);
                    retry_delay(attempt).await;
                    continue;
                }
                return Err(error);
            }
            let payload = serde_json::from_str::<GraphqlResponse<T>>(&body).map_err(|error| {
                GithubStatsError::InvalidResponse {
                    message: error.to_string(),
                }
            })?;
            if let Some(errors) = payload.errors {
                return Err(graphql_error(errors));
            }
            return payload
                .data
                .ok_or_else(|| GithubStatsError::InvalidResponse {
                    message: "missing GraphQL data".to_owned(),
                });
        }
        Err(last_error.unwrap_or_else(|| GithubStatsError::Remote {
            kind: RemoteErrorKind::UpstreamUnavailable,
            message: "request failed after retries".to_owned(),
        }))
    }

    async fn request_body(
        &self,
        request: Request<Full<Bytes>>,
    ) -> Result<Option<String>, GithubStatsError> {
        let connector = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_only()
            .enable_http1()
            .enable_http2()
            .build();
        let client = Client::builder(TokioExecutor::new()).build(connector);
        let response = match client.request(request).await {
            Ok(response) => response,
            Err(_) => return Ok(None),
        };
        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .map_err(|error| GithubStatsError::Remote {
                kind: RemoteErrorKind::UpstreamUnavailable,
                message: error.to_string(),
            })?
            .to_bytes();
        let body = String::from_utf8(body.to_vec()).map_err(|error| {
            GithubStatsError::InvalidResponse {
                message: error.to_string(),
            }
        })?;
        if !status.is_success() {
            return Ok(Some(format!("{}\n{}", status.as_u16(), body)));
        }
        Ok(Some(body))
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        token: &str,
        url: &str,
    ) -> Result<T, GithubStatsError> {
        let mut last_error = None;
        for attempt in 0..3 {
            let request = Request::builder()
                .method(Method::GET)
                .uri(url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header(USER_AGENT, "github-personal-stats")
                .body(Full::new(Bytes::new()))
                .map_err(|error| GithubStatsError::InvalidResponse {
                    message: error.to_string(),
                })?;
            let Some(body) = self.request_body(request).await? else {
                retry_delay(attempt).await;
                continue;
            };
            if let Err(error) = ensure_success_body(&body) {
                if retryable_body(&error) && attempt < 2 {
                    last_error = Some(error);
                    retry_delay(attempt).await;
                    continue;
                }
                return Err(error);
            }
            return serde_json::from_str::<T>(&body).map_err(|error| {
                GithubStatsError::InvalidResponse {
                    message: error.to_string(),
                }
            });
        }
        Err(last_error.unwrap_or_else(|| GithubStatsError::Remote {
            kind: RemoteErrorKind::UpstreamUnavailable,
            message: "request failed after retries".to_owned(),
        }))
    }

    async fn fetch_user_data_async(
        &self,
        config: &GithubStatsConfig,
    ) -> Result<GithubData, GithubStatsError> {
        let token = self.token(config)?;
        let profile = self
            .post::<ProfileData, _>(
                &token,
                PROFILE_QUERY,
                LoginVariables {
                    login: config.username.as_str(),
                },
            )
            .await?;
        let user = profile.user.ok_or_else(|| GithubStatsError::Remote {
            kind: RemoteErrorKind::NotFound,
            message: format!("user {} not found", config.username),
        })?;
        let repositories = self
            .fetch_owned_repositories(&token, config, user.repositories)
            .await?;
        let mut contributions = BTreeMap::<String, u32>::new();
        let authored_repository_ids = self
            .fetch_authored_repository_ids(&token, config, &repositories.nodes)
            .await?;
        for year in user.contributions_collection.contribution_years.iter() {
            let calendar = self
                .post::<CalendarData, _>(
                    &token,
                    CALENDAR_QUERY,
                    CalendarVariables {
                        login: config.username.as_str(),
                        from: format!("{year}-01-01T00:00:00Z"),
                        to: format!("{year}-12-31T23:59:59Z"),
                    },
                )
                .await?;
            let Some(calendar_user) = calendar.user else {
                continue;
            };
            for week in calendar_user
                .contributions_collection
                .contribution_calendar
                .weeks
            {
                for day in week.contribution_days {
                    contributions.insert(day.date, day.contribution_count);
                }
            }
        }
        let languages = aggregate_repository_languages(
            config,
            &repositories.nodes,
            authored_repository_ids.as_ref(),
        );

        Ok(GithubData {
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
        })
    }

    async fn fetch_owned_repositories(
        &self,
        token: &str,
        config: &GithubStatsConfig,
        mut repositories: RepositoryConnection,
    ) -> Result<RepositoryConnection, GithubStatsError> {
        let mut after = repositories.page_info.end_cursor.clone();
        while repositories.page_info.has_next_page {
            let page = self
                .post::<OwnedRepositoriesData, _>(
                    token,
                    OWNED_REPOSITORIES_QUERY,
                    OwnedRepositoriesVariables {
                        login: config.username.as_str(),
                        after: after.as_deref(),
                    },
                )
                .await?;
            let Some(user) = page.user else {
                break;
            };
            after = user.repositories.page_info.end_cursor.clone();
            repositories.page_info = user.repositories.page_info;
            repositories.nodes.extend(user.repositories.nodes);
        }
        Ok(repositories)
    }

    async fn fetch_authored_repository_ids(
        &self,
        token: &str,
        config: &GithubStatsConfig,
        owned_repositories: &[RepositoryNode],
    ) -> Result<Option<BTreeSet<String>>, GithubStatsError> {
        if config.language_scope == LanguageScope::Owned {
            return Ok(None);
        }

        let mut repository_ids = BTreeSet::<String>::new();
        let mut after = None::<String>;
        loop {
            let page = self
                .post::<AuthoredRepositoriesData, _>(
                    token,
                    AUTHORED_REPOSITORIES_QUERY,
                    AuthoredRepositoriesVariables {
                        login: config.username.as_str(),
                        after: after.as_deref(),
                    },
                )
                .await?;
            let Some(user) = page.user else {
                return Ok(Some(repository_ids));
            };
            for repository in user.repositories_contributed_to.nodes {
                repository_ids.insert(repository.id);
            }
            if !user.repositories_contributed_to.page_info.has_next_page {
                self.add_commit_author_matches(
                    token,
                    config,
                    owned_repositories,
                    &mut repository_ids,
                )
                .await?;
                return Ok(Some(repository_ids));
            }
            after = user.repositories_contributed_to.page_info.end_cursor;
        }
    }

    async fn add_commit_author_matches(
        &self,
        token: &str,
        config: &GithubStatsConfig,
        owned_repositories: &[RepositoryNode],
        repository_ids: &mut BTreeSet<String>,
    ) -> Result<(), GithubStatsError> {
        let mut authors = vec![config.username.clone()];
        authors.extend(config.author_emails.iter().cloned());
        for repository in owned_repositories {
            if repository.is_fork || repository_ids.contains(&repository.id) {
                continue;
            }
            for author in &authors {
                if self
                    .repository_has_author_commit(token, &repository.name_with_owner, author)
                    .await?
                {
                    repository_ids.insert(repository.id.clone());
                    break;
                }
            }
        }
        Ok(())
    }

    async fn repository_has_author_commit(
        &self,
        token: &str,
        name_with_owner: &str,
        author: &str,
    ) -> Result<bool, GithubStatsError> {
        let url = repository_commits_url(name_with_owner, author);
        match self.get_json::<Vec<serde_json::Value>>(token, &url).await {
            Ok(commits) => Ok(!commits.is_empty()),
            Err(GithubStatsError::Remote {
                kind: RemoteErrorKind::NotFound,
                ..
            }) => Ok(false),
            Err(error) => Err(error),
        }
    }
}

impl GithubClient for GithubGraphqlClient {
    fn fetch_user_data(&self, config: &GithubStatsConfig) -> Result<GithubData, GithubStatsError> {
        tokio::runtime::Runtime::new()
            .map_err(|error| GithubStatsError::Remote {
                kind: RemoteErrorKind::UnsupportedConfiguration,
                message: error.to_string(),
            })?
            .block_on(self.fetch_user_data_async(config))
    }
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

fn aggregate_repository_languages(
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

fn http_error_kind(status: u16, body: &str) -> RemoteErrorKind {
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

fn ensure_success_body(body: &str) -> Result<(), GithubStatsError> {
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

fn retryable_body(error: &GithubStatsError) -> bool {
    matches!(
        error,
        GithubStatsError::Remote {
            kind: RemoteErrorKind::UpstreamUnavailable,
            ..
        }
    )
}

async fn retry_delay(attempt: usize) {
    tokio::time::sleep(std::time::Duration::from_millis(300 * (attempt as u64 + 1))).await;
}

fn repository_commits_url(name_with_owner: &str, author: &str) -> String {
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

fn graphql_error(errors: Vec<GraphqlError>) -> GithubStatsError {
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
struct GraphqlRequest<'a, V> {
    query: &'a str,
    variables: V,
}

#[derive(Debug, Serialize)]
struct LoginVariables<'a> {
    login: &'a str,
}

#[derive(Debug, Serialize)]
struct CalendarVariables<'a> {
    login: &'a str,
    from: String,
    to: String,
}

#[derive(Debug, Serialize)]
struct AuthoredRepositoriesVariables<'a> {
    login: &'a str,
    after: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct OwnedRepositoriesVariables<'a> {
    login: &'a str,
    after: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct GraphqlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphqlError>>,
}

#[derive(Debug, Deserialize)]
struct GraphqlError {
    message: String,
    extensions: Option<GraphqlErrorExtensions>,
}

#[derive(Debug, Deserialize)]
struct GraphqlErrorExtensions {
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileData {
    user: Option<ProfileUser>,
}

#[derive(Debug, Deserialize)]
struct OwnedRepositoriesData {
    user: Option<OwnedRepositoriesUser>,
}

#[derive(Debug, Deserialize)]
struct OwnedRepositoriesUser {
    repositories: RepositoryConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileUser {
    login: String,
    name: Option<String>,
    followers: TotalCount,
    repositories: RepositoryConnection,
    pull_requests: TotalCount,
    issues: TotalCount,
    repositories_contributed_to: TotalCount,
    contributions_collection: ContributionsCollection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContributionsCollection {
    contribution_years: Vec<i32>,
    total_commit_contributions: u64,
    total_pull_request_review_contributions: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TotalCount {
    total_count: u64,
}

#[derive(Debug, Deserialize)]
struct RepositoryConnection {
    #[serde(rename = "totalCount")]
    total_count: u64,
    nodes: Vec<RepositoryNode>,
    #[serde(default)]
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryNode {
    id: String,
    name_with_owner: String,
    is_fork: bool,
    stargazer_count: u64,
    languages: LanguageConnection,
}

#[derive(Debug, Deserialize)]
struct AuthoredRepositoriesData {
    user: Option<AuthoredRepositoriesUser>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthoredRepositoriesUser {
    repositories_contributed_to: AuthoredRepositoryConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthoredRepositoryConnection {
    nodes: Vec<AuthoredRepositoryNode>,
    #[serde(default)]
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
struct AuthoredRepositoryNode {
    id: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LanguageConnection {
    #[serde(rename = "totalSize")]
    total_size: u64,
    edges: Vec<LanguageEdge>,
}

#[derive(Debug, Deserialize)]
struct LanguageEdge {
    size: u64,
    node: LanguageNode,
}

#[derive(Debug, Deserialize)]
struct LanguageNode {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CalendarData {
    user: Option<CalendarUser>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarUser {
    contributions_collection: CalendarContributionsCollection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarContributionsCollection {
    contribution_calendar: ContributionCalendar,
}

#[derive(Debug, Deserialize)]
struct ContributionCalendar {
    weeks: Vec<ContributionWeek>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContributionWeek {
    contribution_days: Vec<ContributionDayNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContributionDayNode {
    date: String,
    contribution_count: u32,
}

const PROFILE_QUERY: &str = r#"
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

const OWNED_REPOSITORIES_QUERY: &str = r#"
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

const AUTHORED_REPOSITORIES_QUERY: &str = r#"
query GitHubPersonalStatsAuthoredRepositories($login: String!, $after: String) {
  user(login: $login) {
    repositoriesContributedTo(first: 100, after: $after, contributionTypes: COMMIT, includeUserRepositories: true, orderBy: {field: STARGAZERS, direction: DESC}) {
      nodes { id }
      pageInfo { hasNextPage endCursor }
    }
  }
}
"#;

const CALENDAR_QUERY: &str = r#"
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
