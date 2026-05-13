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
