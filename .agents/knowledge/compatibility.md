# Compatibility Knowledge

## Goal

Compatibility exists to reduce migration cost for users of prior approaches while preserving a simpler native model.

## Rule

Support aliases and familiar parameter names when they map cleanly to the native model. Do not preserve accidental behavior that would weaken architecture, security, or rendering correctness.

## Mapping Records

As parameters are added, record their canonical name, aliases, default, accepted values, and output effect.

| Parameter | Default | Accepted values | Output effect |
| --- | --- | --- | --- |
| `--user` | `octo` | a GitHub login | Whose profile is fetched |
| `--card` | `dashboard` | `dashboard`, `stats`, `languages`, `streak`, `activity`, `status`, plus the `top-langs`, `top-languages`, and `coding-activity` aliases | Selects which card renders |
| `--output` | `profile/github-personal-stats.svg` | a path | Where the rendered card is written |
| `--fixture` | none | a path to sanitized fixture JSON | Renders from fixture data instead of the network |
| `--section`, `--target` | `activity`, `README.md` | a marker name and a path | Which marked README section `update-readme` rewrites |
| `--width`, `--height` | `1000`, `420` | positive integers | Fixed SVG dimensions; regions below 440 wide switch to compact layout |
| `--authored-languages` | off | flag | Restricts language share to owned non-fork repositories the user contributed to |
| `--author-email` | none | comma-separated emails, repeatable | Supplements authorship matching for old commit emails |
| `--hide-language` | none | comma-separated names, repeatable | Drops languages before aggregation |
| `--min-repo-language-share` | `0` | percentage `0`–`100` | Ignores a language in a repository below that share |
| `--heat-window` | `streak` | `streak` or a day count | Ring covers the current streak, or a fixed run of days ending on the latest one |
| `--heat-limit` | none | a day count, or `none` | Caps ring length without changing the reported streak |
| `--heat-shape` | `segmented` | `segmented`, `ticks`, `arcs`, `bands` | Ring geometry; `segmented` switches at the threshold |
| `--heat-threshold` | `100` | a day count | Where `segmented` moves from ticks to averaged arcs |
| `--heat-scale` | `linear` | `linear`, `sqrt`, `log`, `quantile` | Maps a day's contribution count onto the four ramp stops |
| `--heat-color` | `heat-orange` | `heat-orange`, `github-blue`, `forest`, `violet`, `crimson`, `graphite`, one hex value, or four hex values | Ring ramp; one value derives the lighter stops in OkLab |
| `--heat-label` | `{Y}`, or `{X}/last {Y}` for a fixed window | free text over `{X}`, `{Y}`, `{Z}` | Ring centre text; longer templates render smaller |
