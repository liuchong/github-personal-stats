# User Guide

GitHub Personal Stats creates SVG assets for GitHub profile READMEs. The default output is a single dashboard image, and individual cards are available for custom layouts.

<p align="center">
  <img src="../examples/dashboard.svg" alt="Dashboard preview" width="100%" />
</p>

## Setup Overview

1. Add a workflow to your profile repository.
2. Generate one or more SVG files into a tracked directory such as `profile/`.
3. Commit those SVG files from the workflow.
4. Reference the generated SVG from your profile README.

## GitHub Action

Create `.github/workflows/github-personal-stats.yml`:

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

## Private Repository Data

Use a dedicated personal access token when your dashboard should include private repositories. Do not rely on the default `GITHUB_TOKEN` for this purpose: it is scoped to the workflow repository and cannot read every private repository owned by the profile user.

Create one of these tokens:

- Classic PAT: use this template, then create the token with `repo` selected: <https://github.com/settings/tokens/new?description=GitHub%20Personal%20Stats&scopes=repo>
- Fine-grained PAT: use <https://github.com/settings/personal-access-tokens/new>, select the repositories you want counted, and grant read access to metadata and contents.

Save the token in your profile repository as an Actions secret:

```sh
gh secret set PERSONAL_STATS_TOKEN --repo your-login/your-login
```

The workflow should pass only that secret to the Action:

```yaml
token: ${{ secrets.PERSONAL_STATS_TOKEN }}
```

Add a check step before generation so a missing token fails the workflow instead of silently generating public-only data:

```yaml
- name: Check personal stats token
  env:
    PERSONAL_STATS_TOKEN: ${{ secrets.PERSONAL_STATS_TOKEN }}
  run: test -n "$PERSONAL_STATS_TOKEN"
```

Private token access affects repository language share, contribution totals, streaks, and any stats based on private repository metadata. If the token is missing or under-scoped, the dashboard can still render, but the data will be public-only or incomplete.

## Language Scope

By default, the language card counts all owned non-fork repositories. This matches repository language share, but it can include repositories owned by the profile user where most code was written by someone else.

Add `--authored-languages` to count only owned non-fork repositories where the target user has commit contributions:

```yaml
options: --user your-github-login --width 1000 --height 420 --authored-languages
```

If old commits used emails that GitHub no longer associates with the account, add those emails as supplements. The option accepts comma-separated values and can also be repeated:

```yaml
options: --user your-github-login --width 1000 --height 420 --authored-languages --author-email old@example.com,work@example.com
```

This mode still uses only GitHub API data. It does not clone or check out target repositories. The scope is repository-level: once a repository qualifies through GitHub contribution data, username commits, or a configured email match, its repository language sizes are counted. It does not perform per-line authorship analysis.

Hide languages that should not appear in the card:

```yaml
options: --user your-github-login --width 1000 --height 420 --authored-languages --hide-language Ruby
```

`--hide-language` accepts comma-separated values and can also be repeated.

Filter small per-repository language noise without hiding the language everywhere:

```yaml
options: --user your-github-login --width 1000 --height 420 --authored-languages --min-repo-language-share 2
```

`--min-repo-language-share 2` ignores a language in a repository when that language is less than 2% of that repository's language total. If another repository is actually Python-heavy, Python still counts there.

## README Usage

Reference the generated dashboard:

```md
![GitHub Personal Stats](./profile/github-personal-stats.svg)
```

For a richer profile section:

```md
<p align="center">
  <img src="./profile/github-personal-stats.svg" alt="GitHub Personal Stats" width="100%" />
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
cargo run -p github-personal-stats -- generate \
  --user your-github-login \
  --card dashboard \
  --authored-languages \
  --author-email old@example.com,work@example.com \
  --hide-language Ruby \
  --min-repo-language-share 2 \
  --width 1000 \
  --height 420 \
  --output profile/github-personal-stats.svg
```

For a local live preview, export a token with the same permissions:

```sh
GITHUB_TOKEN=YOUR_PERSONAL_STATS_TOKEN cargo run -p github-personal-stats -- generate \
  --user your-github-login \
  --card dashboard \
  --output profile/github-personal-stats.svg
```

Individual cards can use smaller dimensions:

```sh
cargo run -p github-personal-stats -- generate \
  --user your-github-login \
  --card languages \
  --width 520 \
  --height 260 \
  --output profile/languages.svg
```

## Local Preview

The repository includes deterministic showcase data so you can preview changes without network access:

```sh
cargo run -p github-personal-stats -- generate \
  --fixture examples/showcase.json \
  --user showcase \
  --card dashboard \
  --output examples/dashboard.svg
```

Preview individual cards:

```sh
cargo run -p github-personal-stats -- generate --fixture examples/showcase.json --card stats --width 520 --height 260 --output examples/stats.svg
cargo run -p github-personal-stats -- generate --fixture examples/showcase.json --card languages --width 520 --height 260 --output examples/languages.svg
cargo run -p github-personal-stats -- generate --fixture examples/showcase.json --card streak --width 1000 --height 220 --output examples/streak.svg
```

## Coding Activity Section

Update a marked README section:

```md
<!--START_SECTION:waka-->
<!--END_SECTION:waka-->
```

Run:

```sh
cargo run -p github-personal-stats -- update-readme --section waka --target README.md
```

## Visual Notes

- Use the default dashboard when you want a clean profile header without layout drift.
- Use individual cards only when your README needs a custom arrangement.
- Keep generated SVGs committed so profile pages render quickly and do not depend on a live image server.
- Prefer a scheduled workflow cadence such as daily updates; profile stats rarely need minute-level refreshes.
