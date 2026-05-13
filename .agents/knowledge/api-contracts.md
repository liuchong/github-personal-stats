# API Contract Knowledge

## Data Sources

The project will use remote profile, repository, contribution, gist, status, and coding activity APIs. Clients must expose typed responses and typed errors.

## Error Classes

Use explicit categories for authentication failure, permission failure, not found, rate limit, upstream unavailable, invalid response, and unsupported configuration.

## Fixtures

Tests must use sanitized fixtures by default. Live network tests, if added, must be opt-in and must not run in normal CI.

## Current Core Contract

- `GithubStatsConfig` owns username, token environment variable name, card selection, image size, and theme.
- `GithubGraphqlClient` builds typed GraphQL request metadata without reading secrets.
- `GithubClient` is a trait so aggregation tests can use deterministic fixture-backed clients.
- `RemoteErrorKind` classifies authentication, permission, not found, rate limit, upstream unavailable, invalid response, and unsupported configuration failures.
- Fixture parsing is intentionally narrow and deterministic until a full JSON dependency is introduced for real HTTP integration.
