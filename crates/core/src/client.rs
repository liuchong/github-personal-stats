use crate::{
    GithubData, GithubStatsConfig, GithubStatsError, RemoteErrorKind, json::parse_github_fixture,
};

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
            body: user_data_query(&config.username),
        }
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

fn user_data_query(username: &str) -> String {
    format!(
        "query GitHubStatsUserData {{ user(login: \"{username}\") {{ login name followers {{ totalCount }} repositories {{ totalCount }} }} }}"
    )
}
