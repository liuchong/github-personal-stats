use crate::{
    ContributionDay, GithubData, GithubProfile, GithubStatsConfig, GithubStatsError,
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
use std::collections::BTreeMap;
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
        let body = serde_json::to_vec(&GraphqlRequest { query, variables }).map_err(|error| {
            GithubStatsError::InvalidResponse {
                message: error.to_string(),
            }
        })?;
        let request = Request::builder()
            .method(Method::POST)
            .uri(&self.endpoint)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(CONTENT_TYPE, "application/json")
            .header(USER_AGENT, "github-personal-stats")
            .body(Full::new(Bytes::from(body)))
            .map_err(|error| GithubStatsError::InvalidResponse {
                message: error.to_string(),
            })?;
        let connector = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_only()
            .enable_http1()
            .build();
        let client = Client::builder(TokioExecutor::new()).build(connector);
        let response = client
            .request(request)
            .await
            .map_err(|error| GithubStatsError::Remote {
                kind: RemoteErrorKind::UpstreamUnavailable,
                message: error.to_string(),
            })?;
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
            return Err(GithubStatsError::Remote {
                kind: http_error_kind(status.as_u16(), &body),
                message: body,
            });
        }
        let payload = serde_json::from_str::<GraphqlResponse<T>>(&body).map_err(|error| {
            GithubStatsError::InvalidResponse {
                message: error.to_string(),
            }
        })?;
        if let Some(errors) = payload.errors {
            return Err(graphql_error(errors));
        }
        payload
            .data
            .ok_or_else(|| GithubStatsError::InvalidResponse {
                message: "missing GraphQL data".to_owned(),
            })
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
        let mut contributions = BTreeMap::<String, u32>::new();
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

        Ok(GithubData {
            profile: GithubProfile {
                login: user.login,
                name: user.name,
                followers: user.followers.total_count,
                public_repositories: user.repositories.total_count,
            },
            stats: UserStats {
                stars: user
                    .repositories
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
            languages: aggregate_repository_languages(&user.repositories.nodes),
            contributions: contributions
                .into_iter()
                .map(|(date, count)| ContributionDay { date, count })
                .collect(),
        })
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

fn aggregate_repository_languages(repositories: &[RepositoryNode]) -> Vec<RepositoryLanguage> {
    let mut languages = BTreeMap::<String, u64>::new();
    for repository in repositories {
        if repository.is_fork {
            continue;
        }
        for edge in &repository.languages.edges {
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
    } else if status == 404 {
        RemoteErrorKind::NotFound
    } else {
        RemoteErrorKind::UpstreamUnavailable
    }
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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryNode {
    is_fork: bool,
    stargazer_count: u64,
    languages: LanguageConnection,
}

#[derive(Debug, Deserialize)]
struct LanguageConnection {
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
      nodes {
        isFork
        stargazerCount
        languages(first: 10, orderBy: {field: SIZE, direction: DESC}) {
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
