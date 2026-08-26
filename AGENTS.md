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
- One directory per machine, named after the machine, one file per day inside it, plus a manifest. Never write a path another machine also writes.
- A collection is not the record. The local sources forget after about a month, so a collection read today cannot describe an older day. Never write a collection out as the record; merge it in with `records::publish`, which keeps whichever reading of each day saw more.
- Merging readings of the same day takes the larger of each field, never the sum. Summing inflates the record on every run, because every run re-reads the same days.
- Never decide whether to publish by comparing bytes of the whole record. `collected_at` moves on every collection, so that makes a scheduled collector commit forever. Compare file by file, and write the manifest only when it describes something new.
- A published file that cannot be read stops the run. It may be the only surviving copy of a day no source still remembers. Never overwrite what could not be read.
- Anything reporting what has been collected reads the record through `Sink::root`. Never report a fresh collection as though it were the history.
- The storage checkout belongs in the app's runtime state directory. Never put it among the user's projects, and never ask the user to create it by hand.

## Release And Version Rules

- Never change a version number unless the user asks for that change in that message. This covers the workspace `version`, the version a path dependency is pinned to, version constants, tags, and the version in documentation and workflow examples.
- Never publish anything unless the user asks for it in that message: no tag, no GitHub release, no Marketplace listing, no crates.io upload.
- Finishing a feature is not a request to release it. Being asked to release one version is not standing permission to release the next one.
- A publish to crates.io cannot be undone. A version can only be yanked, it stays downloadable, and its number is spent permanently. Treat every publish as irreversible and ask first.
- When work is finished and a release looks warranted, say so and stop there. Choosing the number is the user's decision, not a semver deduction.

## Asking The User

- Every question goes in the reply itself, written as prose. Never use a structured-question tool, a multiple-choice popup, an option list, a radio list, a confirmation dialog, or a form. Those controls hold a few words per option, so the user is shown conclusions with none of the evidence behind them and can only guess or cancel.
- "There are too many options", "the prose would be long", "a popup is clearer", and "the user can pick Other" are not reasons. A popup is never a substitute for the explanation, and an explanation posted after a popup does not repair it.
- A question worth asking carries all of: the specific fact that raised it and what stays blocked without an answer; what the answer changes in scope, behaviour, or acceptance; where the evidence is, by file path with a line number or function name, and the code itself where it is short; for each option, what has to be true for it to be the right one, what it costs, and what it risks; which one is recommended, why, and what would flip that recommendation; and which parts are verified, with how they were verified, against which parts are still assumption.
- Ask only what is the user's to decide: priorities, risk appetite, product judgement, and anything needing access or information the repository does not hold. Anything a command, a log, a test, or the code can settle, settle it before asking.
- Ask the related questions together, with their background, in one pass. Do not extract one narrow answer at a time about the same decision.
- A cancelled popup is not a refusal and not consent. It means the question could not be answered as posed. Re-ask it in prose with what was missing, and say that the earlier attempt did not carry enough to judge.

## Required Reading Order

1. `AGENTS.md`
2. `.agents/program.md`
3. `.agents/current.md`
4. `.agents/index.md`
5. Relevant checklist or playbook for the task
