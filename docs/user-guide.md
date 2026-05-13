# User Guide

GitHub Stats creates SVG assets for GitHub profile READMEs. The default output is a single dashboard image, and individual cards are available for custom layouts.

<p align="center">
  <img src="../examples/dashboard.svg" alt="Dashboard preview" width="100%" />
</p>

## Setup Overview

1. Add a workflow to your profile repository.
2. Generate one or more SVG files into a tracked directory such as `profile/`.
3. Commit those SVG files from the workflow.
4. Reference the generated SVG from your profile README.

## GitHub Action

Create `.github/workflows/github-stats.yml`:

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
      - uses: liuchong/github-stats@v1.0.0
        with:
          card: dashboard
          path: profile/github-stats.svg
          options: --user your-github-login --width 1000 --height 420
      - uses: stefanzweifel/git-auto-commit-action@v5
        with:
          commit_message: "chore: update profile stats"
```

Use a token with enough read permissions if your profile should include private contribution data.

## README Usage

Reference the generated dashboard:

```md
![GitHub Stats](./profile/github-stats.svg)
```

For a richer profile section:

```md
<p align="center">
  <img src="./profile/github-stats.svg" alt="GitHub Stats" width="100%" />
</p>
```

## Card Types

| Card | Output |
| --- | --- |
| `dashboard` | Combined profile dashboard |
| `stats` | Stats and rank card |
| `languages` | Repository language share |
| `streak` | Total contributions, current streak, longest streak |
| `wakatime` | Coding activity card |
| `status` | Service status card |

The aliases `top-langs`, `top-languages`, and `coding-activity` are accepted by the CLI parser.

## Sizing

The default dashboard size is `1000x420`.

```sh
cargo run -p github-stats -- generate \
  --user your-github-login \
  --card dashboard \
  --width 1000 \
  --height 420 \
  --output profile/github-stats.svg
```

Individual cards can use smaller dimensions:

```sh
cargo run -p github-stats -- generate \
  --user your-github-login \
  --card languages \
  --width 520 \
  --height 260 \
  --output profile/languages.svg
```

## Local Preview

The repository includes deterministic showcase data so you can preview changes without network access:

```sh
cargo run -p github-stats -- generate \
  --fixture examples/showcase.json \
  --user showcase \
  --card dashboard \
  --output examples/dashboard.svg
```

Preview individual cards:

```sh
cargo run -p github-stats -- generate --fixture examples/showcase.json --card stats --width 520 --height 260 --output examples/stats.svg
cargo run -p github-stats -- generate --fixture examples/showcase.json --card languages --width 520 --height 260 --output examples/languages.svg
cargo run -p github-stats -- generate --fixture examples/showcase.json --card streak --width 1000 --height 220 --output examples/streak.svg
```

## Coding Activity Section

Update a marked README section:

```md
<!--START_SECTION:waka-->
<!--END_SECTION:waka-->
```

Run:

```sh
cargo run -p github-stats -- update-readme --section waka --target README.md
```

## Visual Notes

- Use the default dashboard when you want a clean profile header without layout drift.
- Use individual cards only when your README needs a custom arrangement.
- Keep generated SVGs committed so profile pages render quickly and do not depend on a live image server.
- Prefer a scheduled workflow cadence such as daily updates; profile stats rarely need minute-level refreshes.
