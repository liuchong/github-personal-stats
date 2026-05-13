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

## Required Reading Order

1. `AGENTS.md`
2. `.agents/program.md`
3. `.agents/current.md`
4. `.agents/index.md`
5. Relevant checklist or playbook for the task
