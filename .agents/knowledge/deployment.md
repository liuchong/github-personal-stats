# Deployment Knowledge

## Action

The Action path downloads a prebuilt release binary for the runner platform, verifies it, and runs it. It must not compile Rust in the consuming repository.

## CLI

The CLI is the stable local interface and should share behavior with the Action and HTTP server.

## Server

The HTTP server should expose the same core renderer through request parameters and should be deployable on common container and cloud platforms.

## Activity Storage

Local activity has to reach whatever renders the cards, and the three ways it can are file, git, and HTTP. See `.agents/knowledge/architecture.md` for the boundary; what matters when deploying:

- The serverless case is the git backend. A repository holds `snapshots/<machine>/<date>.json` and `snapshots/<machine>/manifest.json`, one directory per machine and one file per day, and CI checks it out to render. The repository is storage, not a GitHub arrangement: any git remote the collector and the renderer can both reach qualifies.
- A reader wanting lifetime totals reads each machine's `manifest.json` rollup and does not need the day files at all. A reader wanting a recent window reads the days it names. Both are cheap; reading everything is not required for either.
- The record accumulates and the sources do not. Cursor keeps about thirty days, so the repository holds days that can no longer be collected from anywhere. Treat it as the only copy: it is not reconstructible by re-running the collector.
- A private storage repository read from another repository's workflow needs **no personal access token**. A read-only deploy key on the storage repository, with its private half held as a secret in the rendering repository and passed to `actions/checkout` as `ssh-key`, is sufficient and is narrower than a token. This is verified behaviour, not an assumption.
- A personal access token *is* required for reading the storage repository through the GitHub REST API, because deploy keys authenticate git transport only and not the API. This is the main reason the git backend exists rather than an API client.
- Committing rendered output back into the rendering repository uses the built-in `GITHUB_TOKEN`. A token is only needed to write to a *different* repository.
- `git ls-remote --exit-code` fails on a repository with no commits, so `actions/checkout` of an empty storage repository fails for reasons that look like authentication and are not. Publish once before concluding anything about credentials.

## Release

A release happens only when the user asks for one and says which number it carries. Neither is inferable: finishing a change does not authorise shipping it, and the version is a promise to users rather than a conclusion drawn from the diff. A crates.io upload is the irreversible case — a version can only be yanked, it stays downloadable, and its number is spent for good.

Release assets must include platform binaries and checksums. Smoke tests must verify installation and at least one dashboard generation path.

`checksums.txt` records one archive per line under its published basename, in the layout `shasum -a 256` writes, so `shasum -c` and `sha256sum -c` accept the file unchanged. The publish job hashes the archives from a flat directory and verifies the file it wrote before uploading; it must never rewrite the paths afterwards, because that is how the separator was lost before. The installer reads only the first field, so a broken layout survives every Action run and only surfaces when a person verifies a download.

## Current Action Contract

- `action.yml` calls `scripts/install-action-binary.sh` before invoking `github-personal-stats`.
- The installer downloads from the Action repository release assets, not from the consuming repository.
- If the `version` input is omitted, the installer uses the checked-out Action ref before falling back to `latest`; consuming workflows can still set `version` explicitly to pin the release asset.
- The consuming workflow path must not run Rust build, install, or toolchain setup steps.
- `mode=generate` writes an SVG path; `mode=update-readme` rewrites the marked section in the target file.

## Current Server Contract

- `github-personal-stats-server` listens on `GITHUB_PERSONAL_STATS_ADDR`, defaulting to `127.0.0.1:3000`.
- `/health` returns plain text `ok`.
- `/info` returns workspace metadata as JSON.
- `/api` and `/api/:card` return SVG responses with cache headers.
- `/api/activity-text` previews the coding activity README text output without remote writes.
