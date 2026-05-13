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
- Text output for coding activity is deterministic and independent from SVG rendering.
