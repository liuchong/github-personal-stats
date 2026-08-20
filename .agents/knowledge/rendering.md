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

## Rings Carry Data

Both rings encode a real measurement; neither is decoration.

- The rank ring closes in proportion to the account's ranking percentile, so a top-1% account draws an almost complete circle. `rank_for_stats` returns that percentile in basis points alongside the label, since the label is only a coarse band of the same number.
- The current streak ring is a closed 30-day heat ring: one radial tick per day of `StreakSummary::recent_daily_counts`, shaded along the fire ramp (pale yellow to deep orange) by that day's contribution volume, with zero-contribution days left on the neutral `track`. The ring has no gap and no flame; continuity and intensity come from the shading. When the recent window is empty the ring degrades to a plain track circle.
