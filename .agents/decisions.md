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
