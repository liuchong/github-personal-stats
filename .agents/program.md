# Agent Operating Program

## Purpose

This program keeps work recoverable across AI sessions. Follow it before changing code, documentation, workflows, or release assets.

## Startup Loop

1. Read `AGENTS.md`.
2. Read `.agents/current.md`.
3. Read `.agents/index.md`.
4. Read the checklist or playbook related to the task.
5. Inspect the working tree before editing.
6. Identify whether the task changes architecture, behavior, tests, release, or documentation.

## Implementation Loop

1. Gather the smallest amount of context needed to make a correct change.
2. Choose the existing project pattern once it exists.
3. Make scoped edits.
4. Add or update tests with the change.
5. Run the relevant verification commands.
6. Update durable agent memory if the work changes state, decisions, or known pitfalls.
7. Run the private forbidden-reference scan before staging.

## Review Loop

1. Review changed files as if receiving a pull request.
2. Check boundaries: fetch, aggregate, render, interface, deployment.
3. Check failure behavior and secret exposure.
4. Check tests, fixtures, and snapshots.
5. Check documentation and examples for copy-paste correctness.
6. Record durable findings in `.agents/reviews/` when useful.

## Experiment Loop

Use experiments only when a bounded question cannot be answered by direct implementation.

Each experiment must record:

- Hypothesis.
- Scope.
- Metric.
- Keep criteria.
- Discard criteria.
- Result.
- Follow-up.

Do not leave experimental behavior wired into production paths unless it passes keep criteria and is converted into normal implementation with tests.

## Stop Conditions

Stop and ask the user when:

- Requirements conflict.
- A secret or private data appears in files or logs.
- A structural decision would materially change the committed plan.
- Verification fails in a way that cannot be resolved locally without guessing.

## Output Expectations

Keep final summaries short. Report what changed, how it was verified, and any remaining risk. Do not mention private reference sources in user-facing summaries unless the user explicitly asks outside repository content.
