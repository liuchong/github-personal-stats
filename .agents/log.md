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
