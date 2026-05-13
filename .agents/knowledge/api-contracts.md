# API Contract Knowledge

## Data Sources

The project will use remote profile, repository, contribution, gist, status, and coding activity APIs. Clients must expose typed responses and typed errors.

## Error Classes

Use explicit categories for authentication failure, permission failure, not found, rate limit, upstream unavailable, invalid response, and unsupported configuration.

## Fixtures

Tests must use sanitized fixtures by default. Live network tests, if added, must be opt-in and must not run in normal CI.

## Current Core Contract

- `GithubStatsConfig` owns username, token environment variable name, card selection, image size, and theme.
- `GithubGraphqlClient` performs live GraphQL fetches using the configured token environment variable.
- `GithubClient` is a trait so aggregation tests can use deterministic fixture-backed clients.
- `RemoteErrorKind` classifies authentication, permission, not found, rate limit, upstream unavailable, invalid response, and unsupported configuration failures.
- Fixture parsing remains available for deterministic tests and offline previews.
- Profile workflows should pass a dedicated personal access token when private repository data is expected. The default Actions `GITHUB_TOKEN` is repository-scoped and should not be documented as sufficient for private profile-wide stats.
- Stats use `pullRequests.totalCount`, `issues.totalCount`, pull request review contributions, follower counts, and owner repository stars. Language share aggregates owner non-fork repository language sizes. Streaks use per-year contribution calendars.
