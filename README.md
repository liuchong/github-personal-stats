**Code is cheap, help out with your tokens!**

# GitHub Stats

GitHub Stats is a Rust workspace for generating profile metrics as a unified SVG dashboard. The default output is one dashboard image so README layout is controlled by the renderer instead of by multiple images and HTML alignment rules.

## Goals

- Generate a single dashboard SVG by default.
- Support individual cards for stats, languages, streaks, repositories, gists, status, and coding activity.
- Provide a local CLI, release-binary GitHub Action, and HTTP server.
- Keep rendering dimensions explicit and configurable.
- Keep tests dense enough to protect aggregation, rendering, and deployment behavior.

## Repository Layout

- `crates/core`: shared data model, aggregation, rendering, and configuration.
- `crates/cli`: command-line interface.
- `crates/server`: HTTP interface.
- `.agents`: durable AI development memory and process files.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Coverage is enforced in CI with `cargo llvm-cov` once product logic grows beyond the foundation skeleton.

## CLI

Generate the default dashboard:

```sh
cargo run -p github-stats -- generate --card dashboard --output profile/github-stats.svg
```

Update a marked coding activity section:

```sh
cargo run -p github-stats -- update-readme --section waka --target README.md
```

## GitHub Action

The Action installs a prebuilt release binary and runs it. Consuming workflows do not compile Rust.

```yaml
name: GitHub Stats

on:
  workflow_dispatch:
  schedule:
    - cron: "0 0 * * *"

jobs:
  generate:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v5
      - uses: liuchong/github-stats@v1
        with:
          card: dashboard
          path: profile/github-stats.svg
      - uses: stefanzweifel/git-auto-commit-action@v5
        with:
          commit_message: "chore: update profile stats"
```

## License

This project is licensed under 1PL. See [`LICENSE`](LICENSE).
