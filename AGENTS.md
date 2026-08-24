# Agent Instructions

This repository is developed through an AI-native workflow. Every agent must read this file, `.agents/program.md`, `.agents/current.md`, and `.agents/index.md` before substantial work.

## Mission

Build a maintainable GitHub profile metrics generator that produces a unified SVG dashboard by default, while also supporting individual cards, command-line use, release-binary Action use, and HTTP deployment.

## Required First Step

The `AGENTS.md` and `.agents/` framework must exist as the first committed project artifact. Product code, Rust workspace setup, CI, license text, release automation, and public README content come after this framework commit.

## Work Boundaries

- Keep architecture, data fetching, aggregation, rendering, CLI, Action, and server concerns separated.
- Prefer long-lived structure over quick local patches.
- Do not add compatibility shims for unpublished branch-only behavior.
- Do not introduce business-code comments. Express intent through names, module boundaries, tests, fixtures, and documentation.
- Documentation may explain architecture and process, but product code should remain self-explanatory.

## Reference Hygiene

- External systems and articles may be studied privately, but repository files must not name them, link to them, copy their text, or include their repository paths.
- Describe learned concepts generically: prior art, reference implementation, profile metrics card, coding activity section, binary Action, persistent agent knowledge.
- Keep the denylist outside the repository. Run a private forbidden-reference scan before every commit and release.
- If a forbidden reference appears in a generated file, remove it before staging.

## Secret Handling

- Never commit tokens, API keys, credentials, `.env` files, private logs, or raw API responses containing secrets.
- Fixtures must be sanitized and deterministic.
- Error messages and snapshots must not expose tokens or private repository data.
- Action, server, and release flows must read secrets only from explicit environment variables or platform secret stores.

## Quality Bar

- Minimum test code to business code ratio: 1:1.
- Target test code to business code ratio: 3:1 where practical for rendering, parsing, aggregation, and compatibility behavior.
- Minimum line coverage after product code exists: 90% for core crates and 85% overall.
- SVG rendering changes require snapshot review.
- Network-facing logic requires fixture-driven tests and explicit error classification.

## Expected Commands

These commands become mandatory once the corresponding project files exist:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo llvm-cov --workspace --fail-under-lines 85
```

Before the Rust workspace exists, use repository inspection, Markdown review, and forbidden-reference scanning as the verification path.

## Commit Rules

- Keep commits small and reviewable.
- The first commit must contain only this agent framework.
- Each functional commit must include relevant tests or test fixtures.
- Update `.agents/log.md` after meaningful work.
- Update `.agents/decisions.md` for durable architecture decisions.
- Update `.agents/knowledge/` when a lesson should survive chat history.

## Branch Naming

- The default branch is `master`, here and in every repository this project creates or configures. Never use `main`, and never leave a tool's default in place because it happened to pick `main`.
- This covers code defaults, help text, configuration files, workflow examples, documentation, and any repository created on the user's behalf. A repository created with `main` must be renamed before anything is built on top of it.

## Activity Storage Rules

- Activity snapshots reach the renderer through one of three backends behind `sink::Sink`: file, git, or HTTP. Which one is used is configuration, never a hardcoded path.
- The git backend is storage, not a GitHub integration. Any git remote both ends can reach qualifies, public internet or not. Do not reach for a host's REST API to do what git transport already does; that trades away self-hosting and forces a token where a read-only deploy key would serve.
- One file per machine, named after the machine. Never write a path another machine also writes.
- Never decide whether to publish by comparing bytes. `collected_at` moves on every collection, so a byte comparison makes a scheduled collector commit forever. Compare the record.
- The storage checkout belongs in the app's runtime state directory. Never put it among the user's projects, and never ask the user to create it by hand.

## Release And Version Rules

- Never change a version number unless the user asks for that change in that message. This covers the workspace `version`, the version a path dependency is pinned to, version constants, tags, and the version in documentation and workflow examples.
- Never publish anything unless the user asks for it in that message: no tag, no GitHub release, no Marketplace listing, no crates.io upload.
- Finishing a feature is not a request to release it. Being asked to release one version is not standing permission to release the next one.
- A publish to crates.io cannot be undone. A version can only be yanked, it stays downloadable, and its number is spent permanently. Treat every publish as irreversible and ask first.
- When work is finished and a release looks warranted, say so and stop there. Choosing the number is the user's decision, not a semver deduction.

## Required Reading Order

1. `AGENTS.md`
2. `.agents/program.md`
3. `.agents/current.md`
4. `.agents/index.md`
5. Relevant checklist or playbook for the task
