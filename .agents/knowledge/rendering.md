# Rendering Knowledge

## Layout Rule

Dashboard layout must be computed inside the renderer. Do not depend on README tables, HTML width attributes, or external CSS to align cards.

## SVG Rule

Every SVG must define fixed `width`, `height`, and `viewBox`. Internal panels should use explicit coordinates, gaps, padding, and typography metrics.

## Snapshot Rule

Rendering changes require snapshot review. Snapshot updates must be intentional and paired with reasoning in the commit or review notes.

## Current Renderer Contract

- `render_card` accepts `CardData` plus `GithubStatsConfig`.
- Dashboard rendering computes all panel coordinates internally.
- Default dashboard uses a two-panel top row and a full-width lower streak panel.
- Small inline SVG icons use a fixed `16x16` viewBox, thin stroke-based drawings (stroke width 1.2–1.4, round caps/joins, no fill), and explicit coordinates so rows keep native SVG alignment without external CSS.
- The current streak hero uses an SVG mask to cut a notch at the top of the ring so a flame icon can visually plug into the ring; the count sits centered inside the ring, with the streak label and date range stacked below.
- Streak tiles switch to a compact layout when the available width is below 640: the hero ring shrinks to radius 26 with smaller typography, side tile labels wrap to two lines, and tile notes use short `Mon D` dates so nothing overflows the tile bounds.
- The streak flame is a single rounded path with a curled tip and a bottom cutout applied through `fill-rule="evenodd"`, drawn in the streak accent orange and sized to overlap the ring top; the mask notch ellipse is wider and taller than the flame base so the ring stroke tapers cleanly behind it.
- Ring strokes stay fine (streak ring 2, rank ring 2.5), language bars stay slim (stacked bar height 6, row bars height 4), the panel accent bar is 2.5 wide and tile underlines 1.5, and the panel drop shadow stays soft (`dy` 4, blur 10, opacity 0.08) so cards read crisp on both light and dark themes.
- Prominent text (panel titles, stat labels and values, side streak numbers) uses `Helvetica Neue` medium (500) with Arial fallback because Arial only provides regular and bold; the streak hero keeps its heavier display weights.
- Text output for coding activity is deterministic and independent from SVG rendering.
