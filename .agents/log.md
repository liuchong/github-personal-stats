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
