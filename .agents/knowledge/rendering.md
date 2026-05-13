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
- Small inline SVG icons use a fixed `16x16` viewBox, theme or data colors, and explicit coordinates so rows keep native SVG alignment without external CSS.
- The current streak hero uses an SVG mask to cut a notch at the top of the ring so a small flame icon can visually plug into the ring; the count sits centered inside the ring, with the streak label and date range stacked below.
- Text output for coding activity is deterministic and independent from SVG rendering.
