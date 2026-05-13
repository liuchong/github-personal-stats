# Deployment Knowledge

## Action

The Action path downloads a prebuilt release binary for the runner platform, verifies it, and runs it. It must not compile Rust in the consuming repository.

## CLI

The CLI is the stable local interface and should share behavior with the Action and HTTP server.

## Server

The HTTP server should expose the same core renderer through request parameters and should be deployable on common container and cloud platforms.

## Release

Release assets must include platform binaries and checksums. Smoke tests must verify installation and at least one dashboard generation path.

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
- `/api/wakatime-text` previews the coding activity README text output without remote writes.
