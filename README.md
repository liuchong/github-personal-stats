**Code is cheap, help out with your tokens!**

# GitHub Personal Stats

Generate a polished GitHub profile dashboard as one SVG. The renderer owns the layout, so your README does not have to fight tables, image heights, or fragile HTML alignment.

<p align="center">
  <img src="./examples/dashboard.svg" alt="GitHub Personal Stats dashboard preview" width="100%" />
</p>

## Why Use It

- One default dashboard for stats, language share, total contributions, current streak, and longest streak.
- Optional individual cards when you want a custom README layout.
- Release-binary GitHub Action, local CLI, and HTTP server deployment path.
- Fixed SVG dimensions with configurable width and height.
- Deterministic rendering backed by fixtures and snapshot tests.

## Card Examples

<p align="center">
  <img src="./examples/stats.svg" alt="Stats card preview" width="49%" />
  <img src="./examples/languages.svg" alt="Languages card preview" width="49%" />
</p>

<p align="center">
  <img src="./examples/streak.svg" alt="Streak card preview" width="100%" />
</p>

## Quick Start

Use the Action from your profile repository and commit the generated dashboard back to `profile/github-personal-stats.svg`.

```yaml
name: GitHub Personal Stats

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
      - name: Check personal stats token
        env:
          PERSONAL_STATS_TOKEN: ${{ secrets.PERSONAL_STATS_TOKEN }}
        run: test -n "$PERSONAL_STATS_TOKEN"
      - uses: liuchong/github-personal-stats@v1.0.0
        with:
          card: dashboard
          path: profile/github-personal-stats.svg
          options: --user your-github-login --width 1000 --height 420 --authored-languages --author-email old@example.com,work@example.com --hide-language Ruby --min-repo-language-share 2
          token: ${{ secrets.PERSONAL_STATS_TOKEN }}
      - uses: stefanzweifel/git-auto-commit-action@v5
        with:
          commit_message: "chore: update profile stats"
```

Do not use the default `GITHUB_TOKEN` when you expect private repository data. It is scoped to the workflow repository and cannot read all private repositories owned by the profile user. Create a dedicated token instead:

- Classic token template: [create a token with `repo` selected](https://github.com/settings/tokens/new?description=GitHub%20Personal%20Stats&scopes=repo).
- Fine-grained token: create one at [Fine-grained personal access tokens](https://github.com/settings/personal-access-tokens/new), select the repositories you want counted, and grant read access to metadata and contents.

Save the token as a repository secret named `PERSONAL_STATS_TOKEN`.

Then add the generated image to your profile README:

```md
![GitHub Personal Stats](./profile/github-personal-stats.svg)
```

## Local Preview

Generate the showcase dashboard from the deterministic example data:

```sh
cargo run -p github-personal-stats -- generate \
  --fixture examples/showcase.json \
  --user showcase \
  --card dashboard \
  --output examples/dashboard.svg
```

Generate an individual card:

```sh
cargo run -p github-personal-stats -- generate \
  --fixture examples/showcase.json \
  --card languages \
  --width 520 \
  --height 260 \
  --output examples/languages.svg
```

Add `--authored-languages` when you want the language card to count only owned repositories where the target user has commit contributions. Add `--author-email` with comma-separated values, or repeat it, for historical commit emails that GitHub no longer associates with the user. The default language view still counts all owned non-fork repositories.

Add `--hide-language Ruby` when repository-level language data includes languages you do not want to display. The option accepts comma-separated values and can also be repeated.

Add `--min-repo-language-share 2` to ignore languages that make up less than 2% of an individual repository before the global language share is calculated.

## Documentation

- [User Guide](docs/user-guide.md): Action setup, CLI usage, card types, sizing, and README patterns.
- [Deployment Guide](deploy/README.md): HTTP server, container, and Kubernetes deployment notes.
- [Vercel Notes](deploy/vercel/README.md): lightweight serverless deployment considerations.

## Repository Layout

- `crates/core`: shared data model, aggregation, rendering, and configuration.
- `crates/cli`: command-line interface.
- `crates/server`: HTTP interface.
- `examples`: deterministic showcase data and generated SVG previews.
- `.agents`: durable AI development memory and process files.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Coverage is enforced in CI with `cargo llvm-cov`.

## License

This project is licensed under 1PL. See [`LICENSE`](LICENSE).
