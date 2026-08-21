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
      - uses: liuchong/github-personal-stats@v1.3.0
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

## Themes

`--theme` picks the palette: `light` (default), `dark`, or `transparent`. An unrecognised name fails the run rather than quietly rendering the default.

```yaml
options: --user your-github-login --theme dark
```

`transparent` drops the background but keeps the dark text of the light palette, so it belongs on a light surface. For a dark surface use `dark`.

### Following the reader's colour scheme

GitHub honours `<picture>` in a README, so generate one card per surface and let the browser choose. Add a second generate step:

```yaml
      - uses: liuchong/github-personal-stats@v1.3.0
        with:
          card: dashboard
          path: profile/github-personal-stats.svg
          options: --user your-github-login --theme light
          token: ${{ secrets.PERSONAL_STATS_TOKEN }}
      - uses: liuchong/github-personal-stats@v1.3.0
        with:
          card: dashboard
          path: profile/github-personal-stats-dark.svg
          options: --user your-github-login --theme dark
          token: ${{ secrets.PERSONAL_STATS_TOKEN }}
```

Then reference both, keeping the light file as the fallback for clients that ignore the media query:

```html
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="./profile/github-personal-stats-dark.svg" />
  <img src="./profile/github-personal-stats.svg" alt="GitHub Personal Stats" width="100%" />
</picture>
```

Colour scheme queries inside the SVG are not a substitute. GitHub serves the file as an image through its own proxy, so a single self-switching SVG cannot be relied on; two files and `<picture>` is the supported path.

### What the dark palette changes

A heat ramp encodes intensity as distance from the background, so the same stops cannot serve both surfaces. On a light card the quiet end is pale and recedes; on a dark card that same pale stop is the brightest thing in the ring, which makes a one-commit day outshout a fifty-commit day. The built-in palettes and single-colour ramps therefore turn around on `dark`: the quiet end sinks to just above the ring track and the busy end climbs.

| `--theme light` | `--theme dark` | `dark` with light stops forced |
| --- | --- | --- |
| <img src="images/heat-ring/theme-light.svg" alt="Ring on a light card" width="164" /> | <img src="images/heat-ring/theme-dark.svg" alt="Ring on a dark card" width="164" /> | <img src="images/heat-ring/theme-dark-explicit.svg" alt="Light stops on a dark card" width="164" /> |
| Quiet days recede, busy days read orange | Quiet days recede into the surface, busy days glow | The quiet majority dominates and the ring reads inside out |

Four explicit stops are always taken exactly as given, on every theme, which is how the third example above is produced. If you spell out stops and also want dark support, spell out a second set for the dark card.

## Panel Content

Each panel decides what it reports. The defaults are what the card has always shown, so none of this needs configuring.

### Stats rows

Six figures are collected, and all six already feed the rank score. Four are listed by default; `--stat-rows` chooses which ones appear and in what order.

| Name | Row |
| --- | --- |
| `stars` | Total Stars |
| `commits` | Commits |
| `prs` | Pull Requests |
| `issues` | Issues |
| `reviews` | Reviews |
| `repos` | Contributed To |

| default | `--stat-rows stars,commits,prs,issues,reviews,repos` | `--stat-rows reviews,repos` |
| --- | --- | --- |
| <img src="images/panels/stats-default.svg" alt="The four default stats rows" width="230" /> | <img src="images/panels/stats-all-six.svg" alt="All six stats rows" width="230" /> | <img src="images/panels/stats-two.svg" alt="Two stats rows" width="230" /> |
| Stars, commits, pull requests, issues | Reviews and repositories contributed to as well | A short list spreads out |

Rows share the height the panel has, so a longer list sits closer together, down to a floor that keeps two rows from touching. A list must name at least one row and cannot name the same row twice.

### Language rows

`--language-rows` sets how many languages the panel lists, from 1 to 8, defaulting to 6. The dashboard splits them across two columns.

| default | `--language-rows 3` |
| --- | --- |
| <img src="images/panels/languages-default.svg" alt="Six languages" width="360" /> | <img src="images/panels/languages-three.svg" alt="Three languages" width="360" /> |

The bar above the list covers the languages the list shows, so when a profile has more languages than the panel lists, the remainder stays as visible track rather than being folded into the last segment.

### Streak panels

The panels either side of the ring each report one figure with the dates that figure covers. `--streak-sides` names the left and right one.

| Name | Panel |
| --- | --- |
| `total` | Total contributions, all years |
| `longest` | Longest streak, with its range |
| `current` | Current streak, with its range |
| `active` | Days with at least one contribution |

| default | `--heat-window 30 --streak-sides active,current` |
| --- | --- |
| <img src="images/panels/streak-default.svg" alt="Total contributions and longest streak" width="470" /> | <img src="images/panels/streak-active-current.svg" alt="Active days and current streak" width="470" /> |

`current` is worth putting in a panel when the ring is not already reporting it, which is the case for a fixed window: the ring then covers the last N days while the panel keeps the streak itself. On the default streak window the ring centre is already the current streak, so a `current` panel repeats it.

A panel cannot be left empty and the two panels cannot report the same figure. Regenerate the images in this section with `python3 scripts/render-panel-samples.py` after building the CLI.

## Heat Ring

The dashboard and streak cards draw the current streak as a ring, where each node is one day and its depth is that day's contribution count. The ring covers exactly the days the number in its centre reports, so the two never disagree.

Everything below is optional. The defaults need no configuration.

### Window

The window decides how many days the ring covers. `streak` follows the current streak, so every node is an active day. A fixed window covers a set number of days ending on the latest one, including quiet days.

| `--heat-window streak` (default) | `--heat-window 30` | `--heat-limit 30` |
| --- | --- | --- |
| <img src="images/heat-ring/window-streak.svg" alt="A 117 day streak" width="164" /> | <img src="images/heat-ring/window-fixed-30.svg" alt="A fixed window of 30 days" width="164" /> | <img src="images/heat-ring/window-limit-30.svg" alt="A streak capped at 30 days" width="164" /> |
| 117 active days, one node each | 30 days, quiet ones in grey | The last 30 days of a 117 day streak |

A streak is uncapped by default. `--heat-limit` caps how many days the ring draws without changing the streak itself, which is useful once a streak grows past the point where the ring stays informative. The date line under the ring always describes what the ring covers, not the whole streak.

### Centre label

Three counts are available, and a template places them:

| Placeholder | Meaning |
| --- | --- |
| `{X}` | Days inside the window with at least one contribution |
| `{Y}` | Days the window covers, which is the number of nodes on the ring |
| `{Z}` | The current streak in full, before any limit shortens the ring |

The default template depends on the window: `{Y}` for a streak, where all three counts are the same number, and `{X}/last {Y}` for a fixed window.

| default, streak | default, fixed | `--heat-label "{X}/{Y}/{Z}"` | `--heat-label "{X} of {Y} → {Z}"` |
| --- | --- | --- | --- |
| <img src="images/heat-ring/label-streak.svg" alt="Streak label" width="164" /> | <img src="images/heat-ring/label-active-of-window.svg" alt="Active days over window" width="164" /> | <img src="images/heat-ring/label-three-counts.svg" alt="Three counts" width="164" /> | <img src="images/heat-ring/label-arrow.svg" alt="Custom arrow template" width="164" /> |

A template is free text, so any separator works. Longer text is set at a smaller size to stay inside the ring; keep templates short if you want the count to stay prominent.

### Shape

Radial ticks read well while they stay separable. Past roughly a hundred days they overlap into a furred edge, so the ring switches to arcs and averages neighbouring days into bands wide enough to see. `segmented` does this automatically and is the default.

| `--heat-shape ticks`, 30 days | `ticks`, 117 days | `arcs`, 117 days | `bands`, 117 days |
| --- | --- | --- | --- |
| <img src="images/heat-ring/shape-ticks-30.svg" alt="Ticks at 30 days" width="164" /> | <img src="images/heat-ring/shape-ticks-117.svg" alt="Ticks at 117 days" width="164" /> | <img src="images/heat-ring/shape-arcs-117.svg" alt="One arc per day at 117 days" width="164" /> | <img src="images/heat-ring/shape-bands-117.svg" alt="Averaged bands at 117 days" width="164" /> |
| Clean and countable | Crowded | One arc per day, under two pixels each | Days averaged into readable bands |

| Value | Behaviour |
| --- | --- |
| `segmented` | Ticks up to the threshold, averaged bands beyond it (default) |
| `ticks` | Always one radial tick per day |
| `arcs` | Always one arc per day, however thin |
| `bands` | Always averaged arcs |

`--heat-threshold` moves where `segmented` switches over. It defaults to 100 days; `--heat-threshold 87` switches at 87 instead.

### Scale

The scale maps a day's contribution count onto the four colour stops. Contribution counts are usually heavy-tailed — many ordinary days and a few large ones — and each scale handles that tail differently.

| `--heat-scale linear` (default) | `sqrt` | `log` | `quantile` |
| --- | --- | --- | --- |
| <img src="images/heat-ring/scale-linear.svg" alt="Linear scale" width="164" /> | <img src="images/heat-ring/scale-sqrt.svg" alt="Square root scale" width="164" /> | <img src="images/heat-ring/scale-log.svg" alt="Logarithmic scale" width="164" /> | <img src="images/heat-ring/scale-quantile.svg" alt="Quantile scale" width="164" /> |
| Faithful to the raw counts, so a few big days dominate and ordinary days stay pale | Keeps ordinary days visible while big days still stand out | Compresses hard, so almost everything reads as busy | Splits the window into four equal shares, giving the most contrast and the least relation to raw counts |

Pick `linear` when you want the ring to be literal about volume. Pick `sqrt` when your busiest days are many times larger than your typical ones and the ring looks washed out.

### Colour

`heat-orange` is the default. Six palettes are built in:

| `heat-orange` | `github-blue` | `forest` | `violet` | `crimson` | `graphite` |
| --- | --- | --- | --- | --- | --- |
| <img src="images/heat-ring/palette-heat-orange.svg" alt="Heat orange" width="140" /> | <img src="images/heat-ring/palette-github-blue.svg" alt="GitHub blue" width="140" /> | <img src="images/heat-ring/palette-forest.svg" alt="Forest" width="140" /> | <img src="images/heat-ring/palette-violet.svg" alt="Violet" width="140" /> | <img src="images/heat-ring/palette-crimson.svg" alt="Crimson" width="140" /> | <img src="images/heat-ring/palette-graphite.svg" alt="Graphite" width="140" /> |

`--heat-color` also takes colours directly. One hex value becomes the busiest stop and the other three are derived from it in OkLab, which keeps the hue instead of fading towards grey; on a dark card the derivation runs the other way, as described under [Themes](#what-the-dark-palette-changes). Four hex values, quietest first, are used exactly as given on every theme.

| `--heat-color "#8250df"` | `--heat-color "#dbe9d5,#a3cf9a,#5aa04f,#1f6f2f"` |
| --- | --- |
| <img src="images/heat-ring/palette-derived.svg" alt="Ramp derived from one colour" width="164" /> | <img src="images/heat-ring/palette-explicit.svg" alt="Four explicit stops" width="164" /> |

### Option reference

| Option | Default | Accepts |
| --- | --- | --- |
| `--heat-window` | `streak` | `streak` or a day count |
| `--heat-limit` | none | a day count, or `none` |
| `--heat-shape` | `segmented` | `segmented`, `ticks`, `arcs`, `bands` |
| `--heat-threshold` | `100` | a day count |
| `--heat-scale` | `linear` | `linear`, `sqrt`, `log`, `quantile` |
| `--heat-color` | `heat-orange` | a palette name, one hex value, or four hex values |
| `--heat-label` | per window | free text with `{X}`, `{Y}`, `{Z}` |

Pass them through the Action the same way as any other option:

```yaml
options: --user your-github-login --heat-window 60 --heat-scale sqrt --heat-color github-blue --heat-label "{X}/last {Y}"
```

### Behaviour worth knowing

- Above the threshold, neighbouring days are averaged so each band stays at least four pixels wide. The ring still spans exactly the days the centre reports, but the bands you can count are fewer than the days.
- A streak window has no quiet days by definition. Only a fixed window shows grey nodes.
- A window longer than the available history pads the missing days as quiet.
- When the streak is zero the ring falls back to a plain track and the centre reads `0`.
- Regenerate the images in this section with `python3 scripts/render-ring-samples.py` after building the CLI.

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

To follow the reader's colour scheme, generate a dark card too and reference both through `<picture>`, as described under [Themes](#following-the-readers-colour-scheme).

## Card Types

| Card | Output |
| --- | --- |
| `dashboard` | Combined profile dashboard |
| `stats` | Stats and rank card |
| `languages` | Repository language share |
| `streak` | Total contributions, current streak, longest streak |
| `activity` | Coding activity card |
| `status` | Service status card |

The aliases `top-langs`, `top-languages`, and `coding-activity` are accepted by the CLI parser.

## Command Line

`github-personal-stats help` lists every command and option with its default. `--help` and `-h` work in the same place, including after a command, so `generate --help` prints the same page without rendering anything. The Action takes these same options through its `options` input.

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
cargo run -p github-personal-stats -- generate --fixture examples/showcase.json --user showcase --card stats --width 520 --height 260 --output examples/stats.svg
cargo run -p github-personal-stats -- generate --fixture examples/showcase.json --user showcase --card languages --width 520 --height 260 --output examples/languages.svg
cargo run -p github-personal-stats -- generate --fixture examples/showcase.json --user showcase --card streak --width 1000 --height 220 --output examples/streak.svg
```

`examples/showcase.json` covers 30 days with a few quiet ones, which is what a fixed window needs to show. `examples/streak-117.json` carries a 117 day streak, which is where the ring changes geometry:

```sh
cargo run -p github-personal-stats -- generate --fixture examples/streak-117.json --card streak --width 1000 --height 220 --output /tmp/long-streak.svg
```

## Coding Activity Section

Update a marked README section:

```md
<!--START_SECTION:activity-->
<!--END_SECTION:activity-->
```

Run:

```sh
cargo run -p github-personal-stats -- update-readme --section activity --target README.md
```

## Visual Notes

- Use the default dashboard when you want a clean profile header without layout drift.
- Use individual cards only when your README needs a custom arrangement.
- Keep generated SVGs committed so profile pages render quickly and do not depend on a live image server.
- Prefer a scheduled workflow cadence such as daily updates; profile stats rarely need minute-level refreshes.
