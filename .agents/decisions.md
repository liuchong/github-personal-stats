# Architecture Decisions

## AD-0001: Agent Framework Is The First Artifact

Date: 2026-05-13

Status: Accepted

Context: The project will be developed through repeated AI-assisted sessions. Work needs durable instructions, current state, decision records, knowledge, checklists, and playbooks before product code exists.

Decision: Create `AGENTS.md` and `.agents/` as the first committed artifact. Do not create Rust workspace, CI, license, product README, or business code until this framework is committed.

Consequences: Future sessions can resume from repository files instead of chat history. Product work starts with clear boundaries, review gates, and reference hygiene rules.

Review Date: 2026-06-13

## AD-0002: The Heat Ring Is Configurable, And Its Span Is Its Number

Date: 2026-08-21

Status: Accepted

Context: The ring shipped as a fixed 30-day window while the centre reported the current streak, so a 117-day streak drew 30 nodes above the number 117. The two disagreed. Streaks are also unbounded, and radial ticks stop being separable somewhere around a hundred days, so one geometry cannot serve every length. Contribution counts are heavy-tailed, so one colour scale cannot serve every account either.

Decision: Model the ring as configuration (`HeatRing`) covering window mode, day limit, shape, switch threshold, colour scale, palette, and centre label, and let `aggregation` derive the window so the renderer only draws what it is given. Hold one invariant above all the options: the ring spans exactly the days the centre reports. Default to the current streak with no limit, ticks up to a hundred days and averaged arcs beyond, a linear scale, and the existing orange ramp, so shipped cards keep their look.

Consequences: Every ring choice is answerable from configuration rather than from constants inside the renderer, and `StreakSummary` now carries the window range so captions describe the ring instead of the streak behind it. The averaged geometry breaks the one-node-per-day reading past the threshold in exchange for a legible ring: the span stays truthful while the countable bands do not. Named palettes stay hand-tuned rather than derived, so the default ramp keeps its exact stops; a single supplied colour is instead walked through OkLab, since interpolating towards white in sRGB drains the hue out of the middle stops.

Review Date: 2026-11-21
## AD-0003: A Theme Is A Checked Choice, And A Ramp Resolves Against It

Date: 2026-08-21

Status: Accepted

Context: The renderer had carried light, dark, and transparent palettes from the start, but `GithubStatsConfig::theme` was a bare `String` that fell through to light on any unrecognised value, and the CLI had no flag at all, so only the HTTP server could reach the palettes. Adding a flag on top of that fallback would have shipped an option where a typo silently renders the wrong card. Rendering a dark card also exposed a deeper problem: a heat ramp encodes intensity as distance from the background, so reusing the light stops on a dark surface makes the palest stop the brightest thing in the ring and a one-commit day outshines a fifty-commit day.

Decision: Make `Theme` a checked enum that errors on an unknown name at the library and CLI boundary, leaving the HTTP server to fall back the way it already does for every other unparsable query value. Store the ramp as intent — `HeatRamp::Named`, `Seed`, or `Explicit` — and resolve it to stops against the theme at render time, turning named palettes and seed derivations around on dark while honouring four explicit stops verbatim. Carry the resolved `Theme` on `RenderTheme::kind` so a palette holder can resolve theme-dependent colours without another parameter.

Consequences: Option order stops mattering, since `--theme` and `--heat-color` no longer race to resolve colours, and one configuration can render both surfaces. Light output is unchanged, because named ramps still read their hand-tuned table. Callers who spell out four stops and want dark support must spell out a second set, which is the price of taking explicit colours literally. Two cards plus a README `<picture>` element is the supported way to follow a reader's colour scheme; a self-switching SVG is not, because GitHub serves the file as a proxied image.

Review Date: 2026-11-21

