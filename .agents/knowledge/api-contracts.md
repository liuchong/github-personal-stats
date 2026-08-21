# API Contract Knowledge

## Data Sources

The project will use remote profile, repository, contribution, gist, status, and coding activity APIs. Clients must expose typed responses and typed errors.

## Error Classes

Use explicit categories for authentication failure, permission failure, not found, rate limit, upstream unavailable, invalid response, and unsupported configuration.

## Fixtures

Tests must use sanitized fixtures by default. Live network tests, if added, must be opt-in and must not run in normal CI.

## Current Core Contract

- `GithubStatsConfig` owns username, token environment variable name, card selection, image size, theme, language scope, and the heat ring configuration.
- `GithubGraphqlClient` performs live GraphQL fetches using the configured token environment variable.
- `GithubClient` is a trait so aggregation tests can use deterministic fixture-backed clients.
- `RemoteErrorKind` classifies authentication, permission, not found, rate limit, upstream unavailable, invalid response, and unsupported configuration failures.
- Fixture parsing remains available for deterministic tests and offline previews.
- Profile workflows should pass a dedicated personal access token when private repository data is expected. The default Actions `GITHUB_TOKEN` is repository-scoped and should not be documented as sufficient for private profile-wide stats.
- Stats use `pullRequests.totalCount`, `issues.totalCount`, pull request review contributions, follower counts, and owner repository stars. Language share aggregates owner non-fork repository language sizes. Streaks use per-year contribution calendars.
- `--authored-languages` keeps language aggregation API-only and restricts language share to owned non-fork repositories that match contribution data, username commit author data, or configured `--author-email` supplements from the REST commits API. `--author-email` accepts comma-separated values and can be repeated. It is repository-level filtering, not per-line authorship analysis.
- `--hide-language` removes named languages before aggregation. It accepts comma-separated values and can be repeated.
- `--min-repo-language-share` filters languages below the configured per-repository percentage before language aggregation, using GraphQL `languages.totalSize`.
- `--heat-window`, `--heat-limit`, `--heat-shape`, `--heat-threshold`, `--heat-scale`, `--heat-color`, and `--heat-label` populate `HeatRing`. They reach the Action through its existing `options` passthrough rather than through dedicated inputs, so the Action surface stays fixed as ring options grow.
- `--theme` populates `GithubStatsConfig::theme` through the same passthrough. `HeatRing::ramp` holds a `HeatRamp` intent rather than resolved colours, so option order does not matter and one config can render both surfaces.
- The crates are published, so the library surface is a compatibility promise: `HeatRamp` is opaque and constructed only through `parse` or `Default`, and any new public type must not be able to describe a state its methods cannot serve.
- Workspace `version` tracks the release tags, and `info` reports it, so a bump belongs in the same commit as the release. `github-personal-stats-core` is declared once in `[workspace.dependencies]` with the version its dependents must publish against. `cli` and `server` can only be packaged after `core` reaches the index, which fixes the publish order.
- Two cards plus a README `<picture>` element is the supported way to follow a reader's colour scheme. A single SVG carrying its own colour-scheme query is not, because GitHub serves the file as a proxied image.

## Aggregated Field Conventions

- `AggregatedStats::rank` and `AggregatedStats::percentile_basis_points` come from the same weighted percentile model. The basis-point value is the position in the ranking distribution where lower is better, so `100` means the top 1% of accounts and the letter label is only a coarse band of that number. Renderers that show progress must therefore invert it.
- `StreakSummary::recent_daily_counts` is the window the heat ring draws, oldest first, sized by `HeatRing::span`. A streak window ends on the streak's last day and is therefore free of quiet days; a fixed window ends on the most recent day present in the calendar rather than on today, and zero-fills days the calendar omits. A limit shortens the window without touching `current`. The vector is empty when there is no contribution data or when the streak is zero.
- `StreakSummary::window_start` and `window_end` bound that same window, so captions can describe the ring rather than the streak behind it. They match `current_start` and `current_end` only when the window covers the whole streak.
- Percentage-style fields use basis points rather than floats so aggregation stays exactly comparable and snapshot output stays deterministic.
