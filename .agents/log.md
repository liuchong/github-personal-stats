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
## Storage Is A Git Repository, Not A GitHub One

Local activity is collected where the records are and rendered somewhere else, so a snapshot has to travel. Three backends sit behind `sink::Sink`: a file, a git repository, and an HTTP endpoint left unimplemented on purpose. The git one carries the serverless case.

Two things were got wrong first and are worth remembering. Publication was decided by comparing the file, which meant a collector on a timer committed every time it woke, because `collected_at` moves whether or not anything happened; comparing the record instead is what makes a quiet night leave no trace. And the checkout was something the user had to make by hand, which put storage among their projects; the sink clones and rebases it itself now, in the app's runtime directory, where it can be deleted without consequence.

Reading a private storage repository from another repository's workflow was assumed to need a personal access token. It does not: a read-only deploy key, with its private half a secret in the rendering repository, is narrower and sufficient. This was verified rather than reasoned about — the key clones and is refused a push, and a real run in the profile repository checked the storage out with no token present. The distinction that matters is transport: a deploy key authenticates git and not the REST API, so had the design read storage through the API a token would have been unavoidable. That is a second reason to speak git, on top of not binding the project to one host.

One trap left a false signal along the way. `actions/checkout` fails against a repository with no commits, because `git ls-remote --exit-code HEAD` finds no refs, and the failure looks exactly like a permission problem. Publish once before drawing conclusions about credentials.

## A Green Build Was Bought By Testing What Faces Strangers

CI had been red since the storage work landed, for two reasons that both said something.

The first was that four sink tests failed on the runner with `Author identity unknown`. The tests were not wrong: a runner has no `user.email`, and neither does a fresh machine. A collector that runs on a timer in the background has no business requiring the user to have configured a global git identity first, so the sink now passes an identity per command, keeping a configured one where there is one and naming the tool where there is not. Nothing is written into the checkout's configuration. This is reproduced locally with `GIT_CONFIG_GLOBAL=/dev/null`, which is worth knowing: changing `HOME` to get the same effect breaks Cargo instead.

The second was coverage, at seventy-two percent against a gate of eighty-five. Raising the gate's exclusions until it passed would have been the quick answer and the wrong one, because the uncovered code was the code most worth covering: the HTTP parser that faces whatever is sent to the port, the panel that renders a page, and the reader of Cursor's database, at seven percent. Those now have tests, and three things came out of writing them that would not have come out of reading the code.

The parser took a `&TcpStream`, which is why it had no tests; it takes `impl Read` now and is exercised against truncated, oversized, and malformed input directly. Writing those tests found that an oversized body was returned as an empty one, so the daemon answered "malformed" when it meant "too large" — it is a refusal with a 413 now. The body was never read either way, so the security property held; only the explanation was wrong.

Testing the Cursor reader meant building a database with its schema, which is the only honest way to test a reader of someone else's data. Three expectations in the first draft of those tests were wrong rather than the code: `commitDate` holds git's own date string and not an ISO one, a sitting is measured from its first moment to its last so a gap is not counted, and time falls to the language being worked on when each stretch began, which means the moment that closes a sitting claims none. All three are now written down as tests, because each is the kind of thing that would otherwise be rediscovered by someone doubting a number on a card.

What remains excluded is the `main.rs` of each binary, which is argument parsing and wiring. That is only defensible while they stay thin, and the daemon's had grown to two hundred and thirty-nine lines of report-building; that reading moved into `status`, where it is tested against a plugin loaded but idle, an editor heard from that never announced itself, and a clock running ahead. Anything substantial appearing in a `main.rs` again is a sign it is in the wrong place, not a sign the gate needs another exclusion.

## The Record Was Being Quietly Overwritten By What The Sources Could Still Remember

A question about why the published record was still one file per machine turned up something worse than the layout.

Cursor's `ai_code_hashes` table holds about thirty days. Measured on this machine: 2026-07-27 to 2026-08-26, 470,932 rows. The published record covered 2026-05-12 to 2026-08-26, and the days before the window told the story plainly — 2026-07-10 held ninety-four thousand committed lines and `agent 0h00m`, zero languages, zero generated lines, while 2026-08-25 held thirteen and a half hours across sixteen languages. Committed lines survive because they come from `scored_commits`, which is keyed by commit and kept longer. Everything else in an aged-out day was already gone.

So today's hours were on a thirty-day fuse. Every run rebuilt from the database and the sink wrote the result as the record, which meant a day's agent time, languages and model breakdown would read as zero a month after it was worked. The help text already promised the opposite — "the snapshot keeps every day it has ever seen" — and `collect` did have a `surviving_days` that carried days over. Two things defeated it. It seeded from `<state>/activity.json`, which is where the *file* sink writes, while this machine publishes through the git sink, so the seed was a side file frozen at the moment the configuration changed. And for any day the fresh reading did cover it did `*slot = reading`, replacing the day wholesale, so a truncated re-reading at the window's edge beat a complete one taken earlier.

The fix separates two things that were being conflated. A **collection** is what the sources say now; it reaches back a month and deliberately never opens the published record, so it cannot shrink something it never read. The **record** is what accumulates, and a collection is merged into it rather than written over it. `collect` no longer seeds itself, and `records::publish` does the merge for every sink, so the file and git backends accumulate identically instead of one of them accumulating by accident.

The merge rule is worth stating because the obvious one is wrong. `absorb` sums, which is right for two machines contributing to one day and catastrophic for re-reading the same day: a collector on a timer would inflate the record all night. Taking the larger of each field is right, and not merely safe, because a day in the past holds a fixed amount of work — a reading of it can be complete or cut short, never larger than the truth. Re-reading a day inside the window reproduces it, re-reading today after more work grows it, and re-reading April keeps what April was published with. What this costs is a correction downwards: a change to how time is counted cannot land on days already published, because the old larger figure outranks it. Those day files have to be deleted to be recollected, and that is written down where someone will hit it.

Splitting days into files was the layout question that started this, and it turned out to be the same answer. A day that is its own file, written once and afterwards only replaced by a fuller reading, makes the accumulation structural rather than a merge step that has to be remembered — and it was exactly the forgetting of that step that caused this. It also pays for itself twice over: a run touches only the days it learned something about, so a reader can fetch a window instead of everything, and a commit can say `Record activity for 2026-08-26` where the whole-file record could only say `Record activity through 2026-08-26T00:44:39Z` above a diff of scattered JSON lines.

What it did not buy is space, and that is worth recording against the instinct to assume it would. Seventy commits of a 76 KB file rewritten whole packed to 61 KB — nine hundred bytes a commit, because each run changed five to ten lines and git deltas that almost perfectly. The single file was never a size problem at this scale and would not have become one for years. Sharding was worth doing for retention, read granularity and a legible history; it was not worth doing to save a repository that was a quarter of a megabyte.

One behaviour was deliberately reversed. An unreadable published file used to be treated as absent and overwritten, on the reasoning that failing to read it should cost a commit rather than the run. That was correct only while the record could be rebuilt from source. Now that it holds days no source remembers, a file that will not parse may be the only copy of them, so the run stops and names the file. A day file from a newer build fails the same way, which is the same answer for the same reason.

Migration was done on the live repository and checked rather than assumed: 102 days before, 102 after, no day missing, and no field smaller in any day. Agent seconds rose by seventy-six, which is the work done while the check ran.


## Collecting Is One Job And Presenting Is Another

The activity work was asked for as a card and built as a pipeline, and the shape it settled into is worth keeping: the collector records facts at their finest grain, and every view is a fold over them.

A fact is a duration or a count with everything known about it attached — the measure it belongs to, the language, the author, the model. Nothing is pre-summed for a particular view. A block of a chart then says which value it wants, what to break it down by, how many rows and what to divide each bar between, and the same fold answers all of them. What this bought was not elegance: it is why hours by model, lines by language and tokens by measure could each be added without touching the collector, and why a fold is read once and folded repeatedly rather than the record being read per block.

Two vocabulary mistakes were made and are worth naming. A day holds several measures of time that overlap — an editor's presence, an agent's work, hours imported from elsewhere — and a block that did not say which one it read let a reader assume it was the only one; every heading names its measure now. And the author of anything not attributable was called `me`, which quietly asserted that a formatter's output and a shell script's writing were typed by a person. It is `unattributed`, which is all the source actually knows.

Alignment turned out to be part of the meaning rather than a finish. A column of durations right-aligned as whole strings lines up the word `mins` and not the digits, so `4 hrs 7 mins` sits under `11 hrs 30 mins` looking broken; the minutes field is padded to two characters instead. In a monospace chart the reader compares by eye down a column, and a column that does not line up is a column that cannot be compared.

## Ninety Per Cent Of What Looked Like A Person Typing Was One Second Of Inventory

The chart reported that nine and three quarter per cent of lines were not written by an agent, on a machine where essentially everything is. The instinct was to argue about the label. The number was wrong, and finding out how took one query.

Of 48,039 lines attributed to `source='human'`, 99.36 per cent arrived within a single second, spread across 135 files and 11 languages. That is not typing; it is the editor taking inventory of a workspace, or backfilling its own table, and recording each existing line as it went. The smaller remainder had the same character at a smaller scale: a formatter rewriting a file, a shell command writing one.

So a second in which unattributed lines appear across more than eight distinct files is discarded, as a sweep rather than as work. The threshold is a judgement, and it is stated where the query is: nobody edits nine files in one second, and a run that legitimately touches many files at once is a tool run, which is exactly what is being excluded. The agent share went from 90.24 to 99.93 per cent, which matches how the work is actually done.

Two general lessons. When a figure contradicts what you know about your own behaviour, the figure is a hypothesis about the data and the fastest route is to ask the data how it is distributed — the timestamp histogram answered in one query what an argument about wording could not. And a source's column names are the source's opinions: Cursor's `source='human'` means "no AI request accounts for this", which is not the same claim as the word suggests, and the code says so where it reads it.

## The Measure Of Being At The Editor Was Measuring The Wrong Person

The editor plugin reported `editor 0h 0m` after thirty-seven hours of continuous work with the window open. The transport was fine — a hand-made pulse was accepted, the token matched, the extension was loaded in the right editor. Zero was what the design asked for.

It watched the things only a person does: saving, switching file, moving the caret. It deliberately ignored documents changing, on the reasoning that an agent editing a file raises that event exactly as typing does, and counting it would put agent work into the measure of a person being present. The reasoning is sound and the conclusion was still useless, because a day spent directing an agent raises none of the events it did watch: the prompt goes into a panel that is not a `TextDocument`, and the edits come back from something that is not you. A more precise answer to the wrong question.

The signal that is true of every way of working is window focus. A pulse now goes out when the window takes focus and every thirty seconds while it keeps it, whoever is typing, filed under whatever file is open. What this gives up is a window left focused while you walk away, which the idle timeout bounds and cannot detect. That error is in one direction and small; reporting nothing for a working day was not.

Three things came out of it. A measure has to be defined by what can be observed about the way work is actually done, not by the cleanest available proxy for the way it used to be done. The `write` flag the pulse had carried since the beginning turned out to have no reader anywhere, so it went — a field nothing consumes is not provenance, it is a claim nobody checks. And a cached token that the daemon has since rotated used to be met with a 4xx, a warning nobody sees and the pulses dropped for ever; a 401 now forgets the token and keeps the pulses for the next attempt.

## A Flag That Parsed, A Record That Loaded, And A Card That Drew Nothing

`generate --card activity --activity-record <dir>` accepted the flag, read the record, and drew `No activity recorded yet`. It had done so since the card was added, because the aggregation returns an empty comparison for that card by design — the profile it aggregates has no activity in it — and nothing on the `generate` path ever built one from the record instead. Every part worked and the wire between two of them was missing.

There was no test. That is the whole explanation, and the sequel proves the point better than the bug does: with the record connected, the top row of the card was a bar with no label holding sixty-four per cent, because most measured hours belong to terminal agents that never say what was being worked on, and the card ranked that nameless share first. The text chart had been fixed for exactly this weeks earlier and the card knew nothing about it — two views filtering one list by their own rules, which is how they come to disagree. Worse, they disagreed on arithmetic too: the card's shares were computed against every measured hour and the chart's against the hours that could be placed, so the same language read 9.4 per cent on one and 26.28 per cent on the other.

The fix is one accessor on the fold that both read, and one figure both declare. Shares are shares of what could be placed, and the remainder is stated in the same words in both places. The tests added are the ones that would have caught each stage: that the card draws from a record at all, that a nameless share is declared rather than ranked, and that a card and a chart of the same measure give the same number.

## Coverage Was Measuring The Decision Not To Test The Network

The gate sat at eighty-five per cent and the workspace at eighty-three, and the largest single block of uncovered lines was two hundred and ninety-five in `core/src/client.rs`. They were all one thing: the connector is built https-only, so it cannot be pointed at a local server, and every line of `post`, `request_body` and the `fetch_*` helpers can only run against GitHub itself.

Padding the percentage elsewhere would have left that permanently dragging the number down, and excluding the file would have hidden how well the rest of it was covered — because everything below those lines, assembling a profile, ranking languages, reading an error body, building a URL, was already tested. The boundary was exactly where the file split naturally, so it was split: `remote.rs` holds what talks to GitHub and is excluded on the same grounds as a binary's `main`, and `client.rs` holds every decision made about what comes back and is measured. Eighty-seven and a half per cent, of the part where a number means something.

The other file worth a mention is the git sink, which publishes the record and was at sixty-three per cent with its shared entry point — the function both the collector and the daemon call to decide what a sink option means — untested altogether. It has tests now, along with a checkout with no history adopting the remote's and a push refused because the remote moved. That was worth doing whatever the gate said.

## A Question Was Asked Through A Control Instead Of In Words

A decision about how lines should be counted was put to the user as a multiple-choice popup. It had been asked that way before and told not to, and the reason it keeps happening is that the tool is there and a list of options looks tidier than a paragraph.

It is not tidier, it is unanswerable. A popup holds a label per option, which means the user is handed conclusions with none of the evidence that produced them: not the file and line the question came from, not what each option costs, not which parts were verified against which were guessed. There are two things they can do with that, guess or cancel, and a cancelled popup then reads as though a decision was declined when in fact the question was never askable as posed.

The rule is written in `AGENTS.md` under asking the user, and on the paths an agent is actually on when the moment arrives: the stop conditions in `.agents/program.md`, which said when to stop and never how to ask; the before-editing entries of `.agents/checklists/implementation.md`; and the output expectations, because a summary that ends in a control the user clicks is the same mistake at the end of the turn.

What the rule asks for is not merely prose instead of a widget. A question has to carry the fact that raised it, what the answer changes, the evidence by path and line, what each option costs and when it is right, a recommendation with what would overturn it, and an honest line between what was checked and what was assumed. Most of the time, writing that out shows the question was answerable without asking, which is the other half of the rule: settle what a command or the code can settle first.

## A Feature With No Picture, And A Card That Only Fitted Wide Pages

Every other thing this project draws has a rendered sample: the ring's four windows and four scales, six palettes, the panels, the tiles. The activity work had none, and its page in the guide had grown into eleven hundred words of text about a card nobody could see. The site's landing page did not mention it at all, so the half of the product that reads how the work was done was invisible to anyone who had not already read the guide to the end.

The samples the other pages use come from `examples/showcase.json`, an invented profile, which is why anybody can regenerate them and why nobody's real figures are in the documentation. Activity needed the same, and a record is a directory of days rather than one file, so `scripts/render-activity-samples.py` writes one. The days are offsets from today rather than dates: a window is *the last thirty days*, so a record pinned to a calendar would slide out of it and every picture would go blank a month after being generated. Offsets keep the numbers identical on every run and keep the pictures true.

Two things came out of finally looking at the thing. The sample's first draft worked on the same languages every day, and the card's bars all had their comparison mark sitting exactly at the bar's end — the feature was drawing nothing, and a sample that flattered the card would have hidden that a real record is the only place the mark means anything. The mix now turns over partway back, so a language picked up recently reads as a bar past its mark.

The second was a defect. At a tile width the two spans were laid side by side in half the card each, and `588 hrs 45 mins` was written straight across `187 hrs 4 mins`; the language names ran into the bars beside them. Both had the same cause: the layout asked how wide the card was, when what mattered was how much room the text needed. A figure is set at one size whatever the card is wide, so the question is whether half the width holds a duration, and a name column measured as thirty per cent of the width is generous at nine hundred and cramped at two hundred and seventy-five. Both now measure what has to fit. The alternative — another width threshold — would have been wrong for the next long total.

One thing that had been true since the card was added and should not have been: `generate --card activity` fetched the profile before deciding it did not need it. So the card that reads only from your own machine could not be drawn without a token or a saved profile, and a workflow rendering it from a storage checkout had to be handed credentials for nothing. It fetches only for the cards that use it now, and a test renders the card with no token and a proxy that would fail any request made.

## A Chart Full Of Periods That Never Said Which Period

Every figure a chart prints is a total or a share of some stretch of time, and the chart named the stretch nowhere. A block said `last 30 days`, which is a length and not a period, and `all time` meant whatever the record happened to reach back to — a fortnight or nine years, with nothing on the page to tell them apart. So the chart now opens with `From: 26 September 2017 - To: 25 August 2026`: the first and last day the record holds work on, spelled out rather than left as digits to be decoded.

The dates are carried per block and stated once as the union. The alternative was one range for the chart, which is wrong the moment a chart holds two measures: hours collected here begin when the collector was installed, hours imported from a tracker used for years begin far earlier, and neither block's range describes what the reader is looking at. The union does.

Which period the line reports follows the data rather than the record's outer edge, and that is deliberate: a chart of lines opens at the first day with lines, not at the first day with anything, because the earlier days hold no lines to have been counted. A record where imported hours reach back to 2017 and line counts only to April reports each honestly depending on what is asked for.

The samples were the other half of the work. `scripts/render-activity-samples.py` claimed its figures were identical on every run, and they were not: the weight of a day came from its weekday, so as the calendar turned, the quiet days moved through the recent window and every number quoted in the guide drifted. Anyone regenerating the samples got a diff on eight blocks of the guide whether or not they had changed anything. The weight now comes from how far back the day is, which keeps the pattern and fixes the figures, and the dates line is quoted once — where it is explained — because it is the one sample line that cannot be stable while the record ends today.

## Two Days Were Wrong: The One It Called Today, And The One It Called The Beginning

Stating the dates immediately showed that neither end of them was right.

The end was a timezone bug that had been there since the fold was written. A day is labelled where the work happened — `crates/collect/src/cursor.rs` asks SQLite for `'localtime'`, the plugin sends its local date — but the window's anchor was UTC. East of Greenwich the record starts a new day hours before UTC does, so between midnight and eight in the morning the newest day was rejected as being in the future and the recent window silently became the thirty days ending yesterday. On a live record at half past three in the morning that was an hour and fifty-three minutes of work missing from every figure, and nothing said so.

Nothing in the standard library knows the machine's offset, and the machine rendering need not be the machine that collected, so the clock cannot answer this. The record can: a day labelled tomorrow by UTC's reckoning is a collector whose calendar has already turned. One day is the whole of the correction, since no offset reaches a full day, which is also what keeps a clock gone wrong from moving the window — a date two days ahead stays excluded.

The beginning was wrong in a subtler way, and only visible because the dates were now on the page. A window's first and last day came from the measure's seconds, so a chart of lines was dated by the hours. On the same record hours reach back to 23 April, kept in transcripts, and lines only to 26 July, kept in an editor database with a few weeks of retention. So a block of lines opened with `From: 23 April` above a total whose earliest evidence was three months later. A window now dates each kind of evidence separately and a block asks for the one it counts, which is the whole point of the line: it has to describe the figures under it, not the record's outer edge.

The samples moved again as a result, and this time for the last time: with the anchor following the record's own labelling, generating them at three in the morning and at noon produce the same figures. `tmp/check.py` compared every sample line the guide quotes against what the renderer produces and found one bar glyph that had been updated by hand and got the run of hashes wrong, which is the argument for comparing rather than editing.

## An Editor That Reports Without A Daemon, And The Day The Daemon Thought It Was

The Emacs mode was asked for as "like wakatime.el", and the first design question was how much of the pipeline it should contain. The recommendation in the request was the most independent one: collect in elisp, aggregate in elisp, and push straight to the data repository through the GitHub API, needing nothing else installed.

That is the version that was not built, and the reasons are worth keeping. The repository's format is defined by `crates/collect/src/records.rs` — day files, a manifest, the keep-fuller merge — so an elisp writer becomes a second implementation of it in a second language, and every schema change then has two places to be right. It would need a write-scoped token sitting in an Emacs config, where `sink.rs` deliberately has none: it shells out to `git` and uses the credentials already there. But the decisive one is arithmetic rather than architecture. Sessions are accumulated across *all* sources at once, in `sessions.rs`, precisely so that an hour spent in Emacs while an agent worked in a terminal is one hour and not two. A plugin that publishes its own aggregate cannot join that; it can only be added to it, which double-bills exactly the way of working this project exists to measure.

So the mode reports moments and nothing else, over two transports: the daemon's `/v1/pulses` when it answers, and the daemon's own append-only journal on disk when it does not. The second is what makes it independent — no daemon, no port, no token, no network — while leaving sessionisation, language attribution, merging and publishing in the one place that already does them for every source. It needed no Rust changes at all, which is the strongest evidence available that it is the shape the architecture already had.

It differs from the VS Code extension in one way, deliberately. That one counts a focused window whether or not anything happens in it, and accepts a walked-away window as a bounded error. Emacs is left in front of you for days, so the same rule would report sleep; presence there is focus *and* input within ten minutes. Under-reporting a session is recoverable in a way that inventing a night is not.

Then the mode found a bug that had nothing to do with Emacs. `daemon status` said `nothing reported today` a minute after a pulse was written, because it asked UTC for today's date and looked for `pulses/<that>.jsonl`, while the journal is named by the local day the plugin observed. In UTC+8 those disagree for the first eight hours of every day — the eight most likely to be worked — so the one command whose entire job is answering "is this working" answered no while it was working.

The first fix guessed the local day from the journal and was wrong: whether a machine's day is ahead of UTC's depends on the hour, not only on the zone, so no timestamp reveals an offset that can be applied to now. The concept was the defect. `reporters` no longer takes a day to look for; it reads a day either side of UTC's and reports each editor on the latest day it actually appears, and the status line names that day. A fact instead of a guess — and it immediately paid for itself by revealing that this machine's VS Code plugin last reported two days ago, which the old wording had been hiding behind `nothing reported today` all along.

Installing it into a real configuration found the mode's own version of the silent zero, before it could cost anything. Presence required `frame-focus-state`, and that function answers `nil` — documented as "definitely known not to be focused" — for a terminal frame whose terminal never reports focus, which includes a plain tty and anything under tmux or screen. A day's work in `emacs -nw` would have been recorded as nothing, which is precisely the failure the VS Code extension already learned once. Focus is now asked only of graphical frames; where it cannot be known, the idle cutoff is the whole test, because whether Emacs was given anything to do is the only signal that exists there.
