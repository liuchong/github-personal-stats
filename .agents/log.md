# Project Log

## [2026-05-13] framework | initialize agent operating structure

Created the first project artifact: root agent instructions and the `.agents/` workspace. This records startup rules, durable memory locations, review gates, and the requirement to keep private reference names and copied source text out of repository content.

## [2026-05-13] foundation | initialize rust workspace

Added the Rust workspace skeleton, 1PL license, CI workflow, ignore rules, foundational README, and compile-tested `core`, `cli`, and `server` crates. Local `cargo fmt`, `cargo test --workspace`, and `cargo clippy --all-targets -- -D warnings` passed. Local coverage could not run because `cargo-llvm-cov` is not installed; CI installs it explicitly.

## [2026-05-13] data-client | add typed config and fixture client

Added typed output selection, image sizing, project config, GitHub request construction, remote error categories, sanitized fixture parsing, and a mock client for deterministic data-client tests. No live network tests were added.

## [2026-05-13] aggregation | add card data aggregation

Added stats score and rank aggregation, language merging and percentage calculation, daily and weekly streak summaries, coding activity summarization, and a shared `CardData` enum for renderers. Added boundary tests for empty data, gaps, weekly dedupe, aliases, and masked coding activity totals.

## [2026-05-13] renderer | add svg and text rendering

Added default dashboard SVG rendering, individual card SVG rendering, fixed `width`/`height`/`viewBox` output, theme selection, coding activity README text rendering, and golden snapshot tests for dashboard, stats, and text output.

## [2026-05-13] cli-action | add generator and binary action

Added CLI `generate` and `update-readme` modes, deterministic CLI tests, composite Action wiring that installs release binaries, release artifact workflow, install script checksum verification, and an Action contract test that rejects Rust build steps in consuming workflows.

## [2026-05-13] server-deploy | add http deployment path

Added a standard-library HTTP server path with `/health`, `/info`, SVG card endpoints, and coding activity text preview. Added server tests, Dockerfile, Kubernetes manifest, and deployment docs. Local Docker build could not run because the Docker daemon was unavailable.

## [2026-05-13] renderer | refine dashboard metrics

Added streak date ranges to aggregated card data, restored a flame marker for current streak rendering, reduced heavy strokes in SVG panels and metric rings, expanded dashboard language rows to six entries, and regenerated the local profile preview with aligned language data. Local `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` passed.

## [2026-05-13] docs | improve user-facing documentation

Reworked the README into a visual landing page, added deterministic SVG examples under `examples/`, and added `docs/user-guide.md` with Action, CLI, card, sizing, and README usage guidance.

## [2026-05-13] release | prepare marketplace release

Added Action branding metadata, taught the release workflow to publish binary archives and a combined checksum file to GitHub Releases, pinned user-facing Action examples to the first release tag, and set explicit release repository context for non-checkout publish steps.

## [2026-05-13] release | publish stable action tag

Published the first release assets, fixed macOS asset name resolution in the installer, and moved user-facing Action examples to the stable release tag that includes the release workflow and installer fixes.

## [2026-05-13] release | add macOS x64 asset

Added a macOS x64 build target to the release matrix so the installer can resolve assets on both Intel and Apple Silicon macOS runners, then moved user-facing Action examples to the complete release tag.

## [2026-05-13] release | use supported macOS Intel runner

Moved the macOS x64 release job from the retired Intel runner label to the supported Intel runner label and advanced the user-facing release tag.

## [2026-05-13] release | consolidate first version

Moved user-facing examples back to the first release tag and consolidated release publishing around a single initial version.

## [2026-05-13] release | choose marketplace name

Renamed the Action display name to a more specific Marketplace-safe title while keeping the first release tag unchanged.

## [2026-05-13] rename | align project identity

Renamed package, binary, crate, documentation, Action, deployment, release asset, and default output references to the new project identity.

## [2026-05-13] renderer | add inline metric icons

Added a small native SVG icon primitive for metric and language rows, updated rendering snapshots, and regenerated example SVG previews.

## [2026-05-13] renderer | refine current streak hero

Recomposed the current streak hero into a torch motif: the ring uses an SVG mask to cut a notch at the top so a redrawn double-layer flame icon visually plugs into the ring, the count sits centered without a redundant unit, and the orange "Current Streak" label and date range stack below the ring. Adjusted ring radius and vertical spacing so the hero fits within both the dashboard streak panel and the standalone 200-pixel-tall streak card. Updated rendering snapshots and regenerated example SVG previews. Local `cargo fmt --all -- --check`, `cargo test --workspace`, and `cargo clippy --all-targets -- -D warnings` passed.

## [2026-05-13] data-client | add live GitHub fetching

Implemented live GitHub data fetching inside the core client using Tokio plus the Hyper client stack. The CLI now uses live GitHub GraphQL data by default and keeps `--fixture` for deterministic tests and offline previews. The live fetch follows the established stats, language, and contribution-calendar field boundaries: profile stats from GraphQL totals, owner repository language aggregation, and per-year contribution calendars for streaks. Verified against `gh api` output for the profile preview, regenerated `examples/liuchong-dashboard.svg`, and ran `cargo test`, `cargo clippy --all-targets -- -D warnings`, and lint checks.

## [2026-05-13] docs | document private token setup

Updated user-facing documentation to require a dedicated personal access token for private repository data, explain why the default Actions `GITHUB_TOKEN` is insufficient for profile-wide private stats, and provide token creation links and workflow validation steps.

## [2026-05-13] data-client | add authored language scope

Added an API-only `--authored-languages` mode that filters language aggregation to owned non-fork repositories where the target user has commit contributions. The default remains owned repository language share.

## [2026-05-13] data-client | supplement authored language emails

Extended authored language filtering with repeatable `--author-email` supplements. The client stays API-only and checks owned repositories through the REST commits API using the username and configured historical emails before counting repository language sizes.

## [2026-05-13] cli | hide selected languages

Added repeatable and comma-separated `--hide-language` CLI filtering so repository-level language noise can be excluded before aggregation.

## [2026-05-13] data-client | paginate owned repositories

Changed live language and star aggregation to paginate all owned repositories instead of only the first 100 repositories. This prevents lower-ranked owned repositories from being omitted from language share calculations.

## [2026-05-13] data-client | add per-repository language threshold

Added `--min-repo-language-share` so small per-repository language slices can be ignored before global language aggregation. This keeps languages like Python visible when they are substantial in a repository while reducing script and test noise in otherwise non-Python repositories.

## [2026-05-13] data-client | tolerate missing pagination metadata

Made repository pagination metadata optional in live GraphQL response parsing so older or partial connection payloads are treated as a single page instead of failing deserialization.

## [2026-05-13] action | pin installer to action ref

Changed the Action installer default from `latest` to the checked-out Action ref so tagged Action runs download matching release assets. Workflows can still override the binary version explicitly with the `version` input.

## [2026-05-24] tests | raise client logic coverage

Investigated repeated CI coverage failures and confirmed the job fails at `cargo llvm-cov --fail-under-lines 85` with overall line coverage at 72.94%, concentrated in `core/src/client.rs`. Added targeted unit tests for language aggregation guard paths, HTTP error classification, response-body success/error parsing, retryability classification, percent encoding, and GraphQL error mapping so newly added live-fetch client logic has explicit branch coverage.

## [2026-05-24] tests | cover renderer variants and client assembly

After the first coverage fix raised total line coverage to 76.33% but still missed the 85% gate, added deterministic tests for streak, coding-activity, status, theme, fallback-color, and zero-total README rendering paths. Extracted live GitHub response assembly into a pure helper and covered profile, stats, authored-language filtering, fork star exclusion, and contribution ordering without requiring network access.

## [2026-05-24] data-client | cover live response parsing

Added deterministic tests for live GraphQL response deserialization, fixture parser failure paths, and CLI command/error paths. The pagination response test exposed that owned repository `pageInfo` was being ignored by serde and therefore defaulting to no next page; fixed the connection field mapping so owned repository pagination metadata is parsed from GraphQL responses.

## [2026-07-21] renderer | refine streak flame and card crispness

Redraw the streak hero flame as a single rounded path with a curled tip and an evenodd bottom cutout, unify flame, ring, and label on the streak accent orange, and enlarge the ring mask notch so the ring tapers cleanly behind the flame. Thin the streak and rank ring strokes, soften the panel drop shadow, and enable geometric precision rendering. Regenerate the dashboard and stats SVG snapshots after visual review of light and dark themes.

## [2026-07-21] renderer | slim strokes across cards

Thinned the streak ring (2), rank ring (3.5), panel accent bar (4), side tile underline (1.5), stacked language bar (height 6), and per-language row bars (height 4) after feedback that card strokes felt heavy. Regenerated both SVG snapshots and confirmed crisp output with 4x rasterized previews.

## [2026-07-21] renderer | lighten typography and redraw row icons

Replaced the filled stat-row icons (star, commit, pull request, issue, code) with thin stroke-based drawings. Moved panel titles, stat labels and values, and side streak numbers from bold weights to `Helvetica Neue` medium (500) since Arial only offers regular and bold; kept the streak hero number and label untouched per feedback. Slimmed the panel accent bar to 2.5 and the rank ring to 2.5. Regenerated both SVG snapshots and confirmed rendering with 4x rasterized previews.

## [2026-07-21] renderer | compact streak tile layout for narrow cards

Fixed side streak tiles overflowing their bounds on narrow streak cards (text spilling past tile edges and card bounds at widths below ~640). Added a compact layout: smaller hero ring and typography, two-line side tile labels, inline unit placement based on value width, and short `Mon D` tile notes. Caught during example image review after the visual refresh; example SVGs must be eyeballed before tagging releases.

## [2026-08-21] renderer | flat hairline visual system with data-driven rings

Replaced the panel, gradient, and drop-shadow chrome with a flat hairline system after feedback that the cards read as heavy and dated. Sections now draw into a `Rect` region so the dashboard and every individual card share one layout path and degrade by region width instead of by card type. Cards sit on a flat background separated by half-pixel hairlines, headers collapse to a single letter-spaced section label, typography moves to the system font stack with tabular numerals declared once on the SVG root, and the accent settles on GitHub blue.

Both rings now carry data instead of decoration. The rank ring closes in proportion to the account's ranking percentile, which required `rank_for_stats` to return the percentile it already computed alongside the label. The current streak ring became a closed 30-day heat ring: one radial tick per day, shaded from pale yellow to deep orange by that day's contribution volume, with quiet days left on the neutral track. This required a trailing `recent_daily_counts` window on `StreakSummary`. The flame was removed; continuity and intensity now come from the data itself.

Added an explicit `on_accent` theme token after the transparent theme exposed that the status badge painted its label with `background`, which is literally `transparent` there and made the label invisible. Regenerated both snapshots and all four example SVGs, and reviewed light, dark, and transparent dashboards plus the narrow streak card as rasterized previews. Workspace line coverage sits at 84.80% against the 85% gate; the shortfall predates this work and lives in `core/src/client.rs` and the untested `server/src/main.rs` socket loop.

## [2026-08-21] renderer | configurable heat ring

Fixed the ring contradicting its own number. The ring covered a fixed 30 days while the centre reported the current streak, so a 117-day streak drew 30 nodes above the number 117. The window is now derived from configuration and the invariant is explicit: the ring spans exactly the days the centre reports.

Added `HeatRing` to the config layer covering window mode (the current streak, or a fixed run of days), an optional day limit, geometry, the threshold that switches geometry, colour scale, palette, and the centre label template. `aggregation` derives the window and now carries `window_start` and `window_end` on `StreakSummary`, because with a fixed window or a limit the date line under the ring was still describing the whole streak. A fixed window also stops calling itself `Current Streak`.

Defaults keep shipped cards intact: the current streak, uncapped, radial ticks up to a hundred days and averaged arcs beyond, a linear scale, and the existing orange ramp. Past the threshold the ring averages days into bands of at least four pixels; one arc per day at that length leaves each under two pixels and reads as stripes rather than a gradient. The averaged ring draws fewer bands than the days it spans, which is the one place the node-per-day reading gives way to legibility.

The centre label takes free text over `{X}` active days, `{Y}` window days, and `{Z}` the streak before any limit. Long templates spilled across the ring, so the centre text now steps its size down to fit. Palettes accept a built-in name, four explicit stops, or one colour walked through OkLab to derive the lighter three, since interpolating towards white in sRGB drains the hue out of the middle stops.

Verified against the real 117-day streak now committed as `examples/streak-117.json`. On that data the default linear scale leaves ordinary days pale and only bursts deep, which is faithful to the counts but not the smooth gradient a long ring suggests; `sqrt` is documented as the alternative. Twenty-three ring illustrations are generated by `scripts/render-ring-samples.py` into `docs/images/heat-ring/` and referenced from the user guide, which now documents every option with a picture of what it does.

## [2026-08-21] docs | align contracts and drop stale examples

Follow-up tidy after the configurable heat ring. `api-contracts.md` still described `recent_daily_counts` as a fixed `RECENT_WINDOW_DAYS` trailing window, a constant that no longer exists; it now documents the configured window, the window range that travels beside it, and the ring options plus their route through the Action's `options` passthrough. `compatibility.md` had an empty parameter-mapping placeholder, so every CLI parameter now has its canonical name, default, accepted values, and output effect recorded in one table, checked against what the CLI actually parses.

Removed `examples/liuchong-dashboard.svg` and `examples/liuchong-dashboard-authored.svg`. Nothing referenced them, they came from real profile data that cannot be regenerated offline, and they showed the pre-refresh ring, so they misrepresented current output.

Documented both preview fixtures in the user guide: the showcase set covers 30 days with quiet days, and `streak-117.json` covers the length where the ring changes geometry. Confirmed the documented regeneration commands reproduce the committed examples and that the ring sample script reproduces all 23 illustrations with no diff. The stats and languages examples had been committed at 480x200 while the guide documented 520x260; regenerating brought the files and the docs back into agreement.

## [2026-08-21] cli | usage output for every command and option

The CLI had no help at all, so nineteen options were discoverable only through the guide. `help`, `--help`, and `-h` now print one usage page, and they work after a command too, so `generate --help` explains itself instead of rendering. An unsupported command prints the same page beside its error rather than a bare failure.

Defaults had been inline literals at their use sites, which would have let the usage text drift away from behaviour on the first change. They are now named constants that both the parser and the usage text read, and a test asserts each documented default appears.

Added a drift guard that extracts every option literal the CLI parses out of its own source and asserts the usage page mentions all of them. It immediately earned its place by catching that `--help` itself was undocumented.

## [2026-08-21] release | v1.1.0 and a verifiable checksum file

Pinned the quick start and user guide to `v1.1.0` and tagged the release. The configurable ring changes visible output on default settings, so it takes a minor bump rather than another patch.

Downloading the published archive to run its own usage output caught that `checksums.txt` could not be verified. The publish step reduced `shasum` output with `sed 's# .*/# #'`, and the greedy match started at the first of the two spaces `shasum` writes between hash and name, collapsing the pair into one. That is not a valid checksum line, so `shasum -c` and `sha256sum -c` both refused the whole file. The Action never noticed because its install script pulls the hash with `awk '{print $1}'`, which is why every release since the first shipped an unverifiable file.

Removed the rewrite instead of correcting the pattern: the publish job now copies the archives into one flat directory and runs `shasum` there, so the names it records are the names it hashed, and it verifies the file it just wrote before uploading. Dropped the per-target `.sha256` files, which were uploaded as build artifacts and never published or read.

Republished the corrected `checksums.txt` on `v1.1.0` after confirming the hashes matched the ones already there, so the only change to the release was the separator.

## [2026-08-21] renderer | themes reach the CLI and the ramp turns around on dark

The renderer had shipped three palettes since the beginning, but `GithubStatsConfig::theme` was a `String` that fell through to light on anything it did not recognise, and the CLI never exposed it, so only the HTTP server could select one. `Theme` is now a checked enum: an unknown name fails at the library and CLI boundary, while the server keeps falling back the way it already does for every unparsable query value. `RenderTheme` carries the theme it resolved from, which is what lets the ring resolve theme-dependent colours without threading another parameter through the section functions.

Rendering the first dark card exposed a worse bug than the missing flag. A heat ramp encodes intensity as distance from the background, so the light stops on a dark surface put the palest colour where the eye reads it as the loudest: a one-commit day outshone a fifty-commit day and the ring was inside out. The ramp is now stored as intent rather than resolved colours — a palette name, a seed, or four explicit stops — and resolves against the theme at render time. Named palettes and seed derivations mirror themselves on dark, sinking the quiet end to just above the ring track and climbing to a brighter busy end. Four explicit stops stay verbatim on every theme, because spelling out colours is already a decision. Storing intent also removed an ordering trap that would have appeared the moment both flags were used: resolving at parse time would have made `--heat-color` before `--theme` mean something different from the reverse.

The dark quiet stop lands at OkLab lightness 0.32, just above the dark ring track at 0.292, so a barely-active day stays distinguishable from a gap without competing with the busy end. A first attempt at 0.40 read as a brown donut.

Light output is byte-identical: the twenty-three existing ring illustrations regenerated with no diff, and the snapshots did not move. Three illustrations were added, including the same ring on a dark card with the light stops forced, which shows the inverted reading the derivation exists to prevent. Documented `--theme` with the `<picture>` recipe for following a reader's colour scheme, and recorded that a single self-switching SVG is not a substitute, since GitHub serves the file as a proxied image.

## [2026-08-21] release | 1.2.1 closes the gaps that publishing would have frozen

Preparing the crates.io release turned up three things worth fixing before any of them became permanent.

`HeatRamp` was a public enum whose `stops` looked up its own variant in the palette table and called `expect` on the result. Every path inside the crate was safe, since only `parse` built one, but a library consumer could write `HeatRamp::Named("nonsense")` and reach the panic — and once published, closing that hole would have been a breaking change. The ramp is now an opaque struct wrapping a private enum, so `parse` and `Default` are the only ways in and every value resolves. The palette table holds byte triples parsed by a `const fn`, which moves a malformed stop from a colour nobody notices to a build failure. Output is unchanged: the twenty-three ring illustrations regenerated with an identical set of colours.

The SVG root declared `role="img"` with nothing to name it, so assistive technology could only announce an unlabelled image. Every card now opens with a `<title>` naming what it shows and the profile it belongs to, referenced by `aria-labelledby`. This is the whole of the snapshot and illustration diff.

The manifest still read `version = "0.1.0"` after three tagged releases, so `info` on a `v1.2.0` binary reported `0.1.0`. Versions now track the tags. Added the description, keywords, categories, and per-crate README that crates.io needs, and moved the core dependency into `[workspace.dependencies]` so its version lives beside the one it must match. Packaging `core` verifies; `cli` and `server` cannot resolve until `core` is on the index, which fixes the publish order rather than being a problem to solve.

Generating the per-card examples straight from the documented commands showed they omitted `--user`, so three of the four committed samples were titled for `octo` while sharing showcase data with a dashboard titled for `showcase`. The commands now pass the user the fixture describes.

## [2026-08-21] feature | the panels report what you choose, and the config struct closes for literals

Three panels were drawing less than the data they already had. Aggregation collected six stats figures and the card listed four, so review counts and repositories contributed to were computed, fed into the rank score, and then dropped. `StreakSummary` carried an active-day count that no card could ever show and a current-streak range reachable only through the ring's centre number. The language panel's row count was a constant. None of this needed new data; it needed the panels to stop hardcoding their contents.

The shape of the flag mattered more than the feature. A flag per figure — `--show-reviews` and its siblings — grows one flag per metric and still cannot say what order the rows go in. An ordered list of names gives order for free, and it can refuse a contradiction: an empty list, an unknown name, and a repeated metric are all errors rather than something quietly collapsed. So `--stat-rows` and `--streak-sides` take lists, `--language-rows` takes a count, and all three validate in core so the CLI, the Action passthrough, and any library caller reject the same input.

Two figures now have somewhere honest to live. `active` reports days with at least one contribution, and `current` is worth a panel exactly when the ring is not already reporting it — a fixed window puts the last N days on the ring and leaves the streak itself to the panel. Each panel dates the figure above it rather than borrowing a neighbour's span.

Putting the settings on `GithubStatsConfig` is a breaking change now that the crate is published, because its fields are public and adding one stops a struct literal compiling. Taking that break later would cost real migrations, so it was taken now, while the only reverse dependency in the index is this workspace's own CLI: the struct is `#[non_exhaustive]` and the builders are the way in. The workspace HTTP server was the first caller to feel it and moved off its literal, which is the same edit any downstream caller would make and the reason to force it while the cost is zero.

Layout had one trap. Keying the column split on how many languages arrived instead of how many rows were configured relaid out any profile with fewer languages than rows — the dashboard snapshot caught it, showing a three-language card moving from one column to two. Splitting on the configured count keeps every existing card where it was.

Default output is unchanged byte for byte against the committed examples. Coverage holds at 87.5% with the renderer at 99.7%, and seven panel illustrations generated by `scripts/render-panel-samples.py` document each option in the user guide.

Released as 1.3.0 rather than 2.0.0, which needed the promise stating rather than assuming. Marking `GithubStatsConfig` `#[non_exhaustive]` breaks literal construction, and strict semver on the library surface would make that a major release. But `core` is on crates.io only because `github-personal-stats` depends on it by version and would not install otherwise — it was never offered as a library, and its nine downloads are this workspace's own installs. Quietly calling a break a minor release would be the fudge; declaring what the number covers is not. The crate's README and description now say `core` is an implementation detail with no API stability promise and tell direct dependents to pin exactly, so the version tracks the CLI, the Action inputs, the option names, and the rendered output, which is what a major release is reserved for. Soundness is not covered by that licence: a reachable panic stays a defect at any version number.

## 2026-08-24 — Tiles A README Can Reflow

A single 1000px dashboard cannot work on a phone. Measuring GitHub's own rendering settled why: the README column is about 846px on a desktop and about 308px on a phone, and an oversized image is scaled down to fit, which takes 12.5px body text under 4px. Making the card responsive is not on the table — GitHub's Markdown honours `<picture>` with `prefers-color-scheme` but not width media queries, and dropping the sizing attributes just hands the decision back to the same downscaling.

What does work is composition. Fixed-width images sit side by side while they fit and wrap when they do not, at their own size either way, so three 275px tiles fill a desktop row and stack into three phone rows, both at 1:1. Reflow is therefore a property of the row, not of the card, and the work was to give a README the pieces to build one.

The streak card was the blocker: three columns leave each figure about 90px at a tile width, which crowded the labels and clipped the dates. A narrow card now gives the ring a full-width row and sits the two figures underneath, which is the only arrangement that fits the date line the ring already draws. Splitting further was worth doing anyway — `heat` draws the ring alone and `metric` draws any single figure — and those reuse the metric names the panels already accept rather than a third list, so `stars` means one thing everywhere. A tile owns its width, so its content centres, which also settled a mismatch inside the narrow streak card where a centred ring sat above left-aligned figures.

Two errors came out of this that are worth remembering. The first was mine and the review caught it: routing to the stacked layout on width alone sent short cards there too, and a 420x200 card drew its values at y=204 on a 200px canvas. Width is not enough to choose a layout that needs vertical room. The regression test now sweeps ten sizes rather than the one tall tile that let it through.

The second only appeared once `--height auto` existed. Fitting a card to its content means measuring a layout from outside it, and the first version measured the three-column streak to its side notes — but the ring's date line hangs lower than the figures beside it, so the fitted card was too short and escaped clipping only because the bottom padding absorbed the overflow. The fix was structural rather than arithmetic: every figure the fitting relies on is now the same named constant the layout draws with, so the two cannot drift. That is also why the caption offsets under the ring stopped being magic numbers.

`auto` is refused for the dashboard, activity, and status cards. They divide a height between sections and have none of their own, and quietly ignoring the flag would be worse than saying so. A single-figure tile too short for three lines drops its date note instead, because that line is supplementary — and an absent note draws no element at all, so it cannot clip.

Two smaller things fell out of composing tiles for real. Padding derived from width leaves the content edges of different widths out of line, so it can be pinned, with the scaling default untouched. And the rank ring was placed by its own radius, but its caption is wider than the ring, so on a tile it reached past the right margin; placement now measures whichever is wider, which leaves cards above about 460px exactly where they were.

`--scale` separates the size a card is laid out at from the size it arrives at, which the vector output makes free: the `viewBox` stays in layout units and only `width` and `height` are multiplied, so a scaled card is the same drawing delivered larger rather than a re-laid-out or resampled one. The multiplier is stored in basis points like the shares elsewhere, which keeps output exact.

The constraint that cannot be solved in code is that the widths in a row have to add up. Past about 825px a tile drops to the next row and leaves a gap beside the ones above it. That is documented rather than detected, because the tool cannot see the column it will land in.

## One Read Behind Fourteen Drawings

Composing a README out of tiles, as the previous change recommended, made each tile fetch the profile again. Rendering was already offline, so the repetition bought nothing and cost the expensive half of the work fourteen times over; a run of fourteen tiles ran out of hourly API allowance at the sixth and committed nothing. Added a `fetch` command that reads a profile once and writes it in the shape `--fixture` already reads, so there is one format on both sides. Writing that shape correctly meant fixing the reader too: it located a field by the first matching key, so the display name has to precede the languages that each carry a name, and it understood no escapes, so a name containing a quote would have been truncated on the way back in. Both directions now handle escapes and round trip, including the hand-written example.

Named the three options that shape a fetch in one place both commands read, and made passing them beside `--fixture` an error rather than a silent no-op, which is the mistake the change would otherwise have introduced. `--hide-language` stays on the render side because that is where it already acted. Taught the action a `fetch` mode, and removed four inputs it declared but never read; a contract test now fails if an input goes unused or a mode has no branch. Measured on a real profile of 194 repositories: the read takes about seven minutes, and the fourteen tiles drawn from it take a third of a second together. Corrected the earlier claim that GitHub ignores width media queries — it strips CSS, but a `<source media>` query is the browser's to evaluate, so a panel can be drawn at both a desktop and a phone width and arrive unscaled in either.

## Releasing Is Not Part Of Finishing

A version was chosen and published without being asked for. The number was deduced from the shape of the diff — a new command reads as a feature, a feature reads as a minor bump — and the release followed the work as if shipping were the last step of building. Neither inference was the agent's to make.

Version numbers and publishing are the user's decision, taken in the message that asks for them. Finishing a feature does not authorise a release, and one authorised release is not standing permission for the next. A crates.io upload is the case that matters most, because it cannot be undone: a version can only be yanked, it stays downloadable, and its number is burnt for good.

The rule is recorded in `AGENTS.md`, and also on every path an agent actually walks while releasing, because the guidance that gets read at that moment is the checklist and the playbook rather than the project preamble: a stop condition in `.agents/program.md`, gating entries at the top of `.agents/checklists/release.md`, a completion check in `.agents/checklists/implementation.md`, step zero of `.agents/playbooks/release-binary.md`, and the release section of `.agents/knowledge/deployment.md`. The playbook was the specific hazard — it opened with "confirm version and changelog source", which reads as an instruction to pick a number.
