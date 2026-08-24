# Implementation Checklist

Before editing:

- Read `AGENTS.md`, `.agents/program.md`, `.agents/current.md`, and `.agents/index.md`.
- Read the relevant playbook.
- Inspect the working tree.
- Confirm the change belongs to the current task.

Before completion:

- Add or update tests for behavior changes.
- Run relevant format, lint, test, coverage, or snapshot checks.
- Update `.agents/log.md`.
- Update knowledge or decisions when durable information changed.
- Run the private forbidden-reference scan.
- Leave the version number alone, and publish nothing. Completing a change is not a release; report that the work is done and let the user decide whether, and as what number, it ships.
