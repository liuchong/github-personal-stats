//! The half of the client that talks to GitHub.
//!
//! Kept apart from `client` because of what can be tested and what cannot. The
//! connector is built `https_only`, so there is no way to point this at a local
//! server: every line here can only run against GitHub itself, and a test that
//! did would be a test of the network. Everything that shapes what comes back —
//! assembling a profile, ranking languages, reading an error body, building a
//! URL — stays in `client`, where it is tested.
//!
//! That is also why this file is excluded from the coverage gate, on the same
//! grounds as a binary's `main`: measuring it would only measure the decision not
//! to test the network, and mixing it in with the rest hid how well the testable
//! part was covered.

use std::collections::{BTreeMap, BTreeSet};
use std::env;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{
    Method, Request,
    header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use serde::{Deserialize, Serialize};

use crate::{
    GithubData, GithubStatsConfig, GithubStatsError, LanguageScope, RemoteErrorKind,
    client::{
        AUTHORED_REPOSITORIES_QUERY, AuthoredRepositoriesData, AuthoredRepositoriesVariables,
        CALENDAR_QUERY, CalendarData, CalendarVariables, GithubClient, GithubGraphqlRequest,
        GraphqlRequest, GraphqlResponse, LoginVariables, OWNED_REPOSITORIES_QUERY,
        OwnedRepositoriesData, OwnedRepositoriesVariables, PROFILE_QUERY, ProfileData,
        RepositoryConnection, RepositoryNode, assemble_github_data, ensure_success_body,
        graphql_error, repository_commits_url, retry_delay, retryable_body,
    },
};

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
        let mut user = profile.user.ok_or_else(|| GithubStatsError::Remote {
            kind: RemoteErrorKind::NotFound,
            message: format!("user {} not found", config.username),
        })?;
        let repositories = self
            .fetch_owned_repositories(&token, config, std::mem::take(&mut user.repositories))
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
        Ok(assemble_github_data(
            config,
            user,
            repositories,
            authored_repository_ids.as_ref(),
            contributions,
        ))
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
