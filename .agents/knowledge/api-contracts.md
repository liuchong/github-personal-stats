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
- `--authored-languages` keeps language aggregation API-only and restricts language share to owned non-fork repositories that match contribution data, username commit author data, or configured `--author-email` supplements from the REST commits API. `--author-email` accepts comma-separated values and can be repeated. It is repository-level filtering, not per-line authorship analysis.
- `--hide-language` removes named languages before aggregation. It accepts comma-separated values and can be repeated.
- `--min-repo-language-share` filters languages below the configured per-repository percentage before language aggregation, using GraphQL `languages.totalSize`.

## Aggregated Field Conventions

- `AggregatedStats::rank` and `AggregatedStats::percentile_basis_points` come from the same weighted percentile model. The basis-point value is the position in the ranking distribution where lower is better, so `100` means the top 1% of accounts and the letter label is only a coarse band of that number. Renderers that show progress must therefore invert it.
- `StreakSummary::recent_daily_counts` is a trailing window of `RECENT_WINDOW_DAYS` daily contribution volumes, oldest first, ending on the most recent day present in the calendar rather than on today. Days missing from the calendar are zero-filled so the window always has a fixed length, days older than the window are dropped, and the vector is empty only when there is no contribution data at all.
- Percentage-style fields use basis points rather than floats so aggregation stays exactly comparable and snapshot output stays deterministic.
