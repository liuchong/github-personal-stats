# Rendering Knowledge

## Layout Rule

Dashboard layout must be computed inside the renderer. Do not depend on README tables, HTML width attributes, or external CSS to align cards.

## SVG Rule

Every SVG must define fixed `width`, `height`, and `viewBox`. Internal panels should use explicit coordinates, gaps, padding, and typography metrics.

## Snapshot Rule

Rendering changes require snapshot review. Snapshot updates must be intentional and paired with reasoning in the commit or review notes.

## Current Renderer Contract

- `render_card` accepts `CardData` plus `GithubStatsConfig`.
- Dashboard rendering computes all coordinates internally.
- Default dashboard uses a two-section top row and a full-width lower streak section.
- Text output for coding activity is deterministic and independent from SVG rendering.

## Region Layout Rule

Every section receives a `Rect` region and derives its own metrics from it, so the dashboard and the individual cards run the same code at any configured size. Sections must degrade on region width, never on card type. A region narrower than `NARROW_WIDTH` (440) switches to the single-column or compact variant: language rows gain per-row tracks, streak metrics shrink, and notes fall back to short `Mon D` dates. The threshold sits just under the dashboard's half-width column so the dashboard stays two-column while the small cards stack.

## Visual System

- Cards are flat: a single background fill, no panels, gradients, or drop shadows. Structure comes from half-pixel hairlines (`stroke-width` 1) on the theme `line` colour.
- Section headers are a single uppercase, letter-spaced label. Do not reintroduce a title plus subtitle pair; it read as redundant.
- `font-family` and `font-variant-numeric: tabular-nums` are declared once on the SVG root and inherited, so text elements only carry size, weight, and fill.
- Inline icons use a fixed `16x16` viewBox and stroke-only drawings at a uniform stroke width of 1.5 with round caps and joins.
- Themes expose `background`, `ink`, `muted`, `line`, `track`, `accent`, and `on_accent`. Content drawn on top of `accent` must use `on_accent`: the transparent theme's `background` is literally `transparent`, so reusing it silently erases foreground text.
- `config::Theme` is a checked enum, not a name string. An unknown theme is a configuration error at the CLI and library boundary; only the HTTP server falls back to the default, matching how it treats every other unparsable query value. `RenderTheme` carries the `Theme` it resolved from in `kind`, so anything holding a palette can resolve theme-dependent colours without a second parameter.
- A theme is a repaint, never a content change. The same card on two themes must carry the same text and the same geometry.

## Rings Carry Data

Both rings encode a real measurement; neither is decoration.

- The rank ring closes in proportion to the account's ranking percentile, so a top-1% account draws an almost complete circle. `rank_for_stats` returns that percentile in basis points alongside the label, since the label is only a coarse band of the same number.
- The current streak ring is a closed heat ring over `StreakSummary::recent_daily_counts`, shaded along the configured ramp by each day's contribution volume, with zero-contribution days left on the neutral `track`. The ring has no gap and no flame; continuity and intensity come from the shading. When the window is empty the ring degrades to a plain track circle.

## Heat Ring Contract

`config::HeatRing` owns every ring choice, and `aggregation` owns the window it describes. The renderer never decides how many days to draw.

- The window and the centre number are the same measurement. `HeatRing::span` returns the day count, `daily_window` fills exactly that many slots, and `{Y}` reports the slot count. Nothing may report a number the ring does not span.
- A streak window anchors on the streak's last day; a fixed window anchors on the latest known day. `window_start` and `window_end` travel on `StreakSummary` so the date line under the ring describes the ring, not the streak behind it. A limit shortens the ring without touching `current`, so `{Z}` still reports the real streak.
- Above `threshold` days, ticks overlap into a furred edge, so the ring switches to arcs and averages days into bands of at least `MINIMUM_BAND_WIDTH`. One arc per day at streak lengths past a hundred leaves each under two pixels, which reads as alternating stripes rather than a gradient. The averaged ring therefore draws fewer bands than the days it spans; the span is what stays honest, not the band count.
- Adjacent arcs extend past their own share so antialiasing cannot leave a seam between them. Arcs paint in window order, so a later band overpaints its neighbour's tail.
- `heat_levels` maps a whole window at once because a quantile scale needs the distribution rather than one day against the peak. Quantile ties fall to the quieter bucket.
- Ramp stops are ordered quiet first, busy last. On a light card quiet is also lightest, but that is a consequence of the surface, not the contract. `HeatRamp` therefore stores intent — a palette name, a seed colour, or four explicit stops — and resolves to stops against the theme at render time, so `--theme` and `--heat-color` can be given in any order.
- A ramp encodes intensity as distance from the background, so the built-in palettes and seed derivations turn around on a dark card: the quiet end sinks to just above the ring `track` and the busy end climbs. Reusing the light stops there makes a one-commit day the brightest thing in the ring and inverts the encoding. Four explicit stops are honoured verbatim on every theme, because a caller spelling out colours has already made that call.
- The dark quiet stop stops just above the dark `track` in OkLab lightness, so a barely-active day stays distinguishable from a gap.
- Real contribution counts are heavy-tailed, so `linear` leaves ordinary days pale and only bursts deep. That is faithful and is the default; `sqrt` is the alternative worth reaching for when the ring looks washed out.
- The centre label is free text over `{X}`, `{Y}`, and `{Z}`. `centre_text_size` steps the size down for longer templates so a configured label can never spill across the ring itself.
- A fixed window is not a streak, so its caption reads `Last N Days` instead of `Current Streak`.
