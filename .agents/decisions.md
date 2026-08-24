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


## AD-0004: A Published Type Cannot Be Allowed To Describe A State It Cannot Serve

Date: 2026-08-21

Status: Accepted

Context: `HeatRamp` was a public enum carrying a palette name, and `stops` resolved that name against the palette table with `expect`. Nothing inside the crate could reach the panic, because `parse` was the only constructor and it only names entries that exist. Publishing to crates.io changes that calculation: the variants become part of the public surface, so `HeatRamp::Named("nonsense")` becomes something a consumer can write, and the panic becomes reachable through documented API. Narrowing it afterwards would be a breaking change.

Decision: Make the ramp opaque — a struct wrapping a private enum — so `parse` and `Default` remain the only ways to obtain one and every value resolves for every theme. Store colours as byte triples inside the variants rather than names to look up or strings to parse, and read the palette table through a `const fn` so a malformed stop fails the build.

Consequences: The panic is unreachable by construction rather than by convention, and the private enum leaves the representation free to change without a major version. Callers lose the ability to match on a ramp's kind, which nothing needed. A wider point holds for the rest of the surface as the crate goes out: an unreachable panic guarded only by which constructors happen to exist is a panic waiting for a consumer.

Review Date: 2026-11-21

## AD-0005: Panel Content Is A Named Metric List, And The Config Struct Stops Being Literal-Constructible

Date: 2026-08-21

Status: Accepted

Context: Six stats figures were aggregated and four were drawn; the streak summary carried an active-day count and a current-streak range that no card could ever show; the language panel hardcoded six rows. Every one of those figures was already computed, so the gap was presentation, not data. The obvious route — one flag per figure, `--show-reviews` and friends — grows a flag per metric and cannot express order. The alternative is a list of metric names, which gives order for free and refuses a contradiction outright. Either way the settings have to live on `GithubStatsConfig`, and that struct now ships on crates.io with public fields, so adding a field breaks literal construction.

Decision: Model panel content as ordered lists of named metrics, `--stat-rows` and `--streak-sides`, plus a plain count for `--language-rows`, all parsed and validated in core so every front end refuses the same mistakes. Reject an empty list, an unknown name, and a repeated metric rather than silently collapsing them. Take the breaking change now: mark `GithubStatsConfig` `#[non_exhaustive]` so the builders become the only way in and every later option is a plain minor release.

Consequences: Default output is unchanged byte for byte, which the committed examples verify. The workspace's own HTTP server had to move off its struct literal onto the builders, which is the change every downstream caller would face and the reason to make it while the only reverse dependency is ours. Layout now derives from the configured count instead of a constant, so the column split had to key on what was asked for rather than what data happened to arrive, or a profile with fewer languages than rows would have silently relaid out. Panel labels stay in the renderer, so a metric name is a stable identifier and the copy beside it can change without breaking a command line.

Review Date: 2026-11-21

## AD-0006: Reflow Comes From Composable Tiles, Not From A Responsive Card

Date: 2026-08-24

Status: Accepted

Context: One 1000px dashboard is unreadable on a phone. GitHub renders a README at about 846px on a desktop and about 308px on a phone and scales an oversized image down to fit, taking 12.5px body text under 4px. The obvious fix — make the card responsive — is not available: GitHub's Markdown honours `<picture>` with `prefers-color-scheme` but not width media queries, and stripping the sizing attributes only hands the decision to the same downscaling. Measurement showed what does work: images with fixed pixel widths sit side by side while they fit and wrap when they do not, at their own size either way.

Decision: Make reflow a composition property rather than a card property. Keep cards fixed-size and give a README the pieces to build a row that wraps: `heat` and `metric` cards so any single figure can stand alone, `--height auto` so a tile carries no dead space, `--padding` so tiles of different widths line their content up, and `--scale` so display size is not tied to layout size. Reuse the existing metric vocabulary for `--metric` rather than inventing a third list of names.

Consequences: A profile composes its own layout and gets phone behaviour for free, with no media queries and no scaling. The cost is that the widths in a row have to add up: past about 825px a tile drops to the next row and leaves a gap, which is a documented constraint rather than something the tool can detect. Fitting a card to its content required every external measurement to be derived from the constant its layout draws with, which immediately exposed a wrong streak height that padding had been absorbing. Two card kinds and four options are now public surface; all four default to previous behaviour, so committed output is unchanged byte for byte.

Review Date: 2026-11-24

Correction, 2026-08-24: the claim above that GitHub honours `<picture>` for colour scheme "but not width media queries" is wrong, and it was wrong about CSS rather than about `<picture>`. GitHub strips `style` and CSS, so a card cannot react to its column; a `<source media>` query is evaluated by the browser and any media query works there, width included. Measured on the rendered page: a 1012px viewport takes the 412px drawing and a 390px one takes the 275px drawing, each at 1:1. The decision stands — fixed-size tiles are still what makes a row wrap — but a panel too wide for a phone can now be drawn at both widths instead of being scaled down, which AD-0007 makes affordable.

## AD-0007: One Fetch Stands Behind Any Number Of Renders

Date: 2026-08-24

Status: Accepted

Context: AD-0006 told profiles to compose a README out of tiles, and following that advice made every tile fetch the profile again. Rendering is already offline — a card is drawn from data and a config, and the client is nowhere in that path — so the repetition bought nothing. It cost a great deal: reading one profile of 194 repositories with `--authored-languages` takes about seven minutes, because attributing a language by who wrote it costs a request per repository per address. Fourteen tiles asking for that fourteen times exhausted the hourly API allowance partway through a run and committed nothing. The guidance was creating the failure.

Decision: Separate reading a profile from drawing it. `fetch` reads once and saves the result in the shape `--fixture` already reads, so there is one file format rather than a second one. Name the options that shape a fetch — `--authored-languages`, `--author-email`, `--min-repo-language-share` — in one place that both commands read, and refuse them next to `--fixture` instead of ignoring them, since a saved profile has already answered them. Leave `--hide-language` on the render side, where it already acted. Do not add a config file to render several cards in one invocation: fourteen renders take a third of a second altogether, so the process count was never the cost.

Consequences: A set of tiles costs one read, whatever the number of cards, themes, and widths, and the render steps need no token because they reach nothing. Drawing a panel at two widths is now a render rather than a fetch, which is what makes AD-0006's correction practical. The writer had to be correct rather than convenient: the existing reader finds a field by the first matching key and understood no escapes, so `name` has to be written before the languages that carry a name of their own, and both sides now handle escapes — a display name with a quote in it is real data, not a hypothetical. The refusal is a behaviour change for a combination that never did anything, and the action's `config`, `target-branch`, `committer`, and `author` inputs went with it: they described a config file and a commit step this action does not have.

Review Date: 2026-11-24
