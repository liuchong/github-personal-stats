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

## Background Processes

Whatever this program starts, it also stops, inside the same turn that started it.

Reviewing the documentation site is the usual temptation here: it needs a real HTTP
root, so it is tempting to leave `python3 -m http.server` running for the next look.
Do not. Past sessions left a stack of them on 8731–8737 plus 8899, and one heredoc
`python3 -` survived its parent shell for ten days, orphaned onto `launchd`, until
the machine was in swap and had to be cleaned up by hand.

- Record the pid when backgrounding (`... & echo $!`) and stop that pid. Do not
  `pkill` on a pattern and hope it matched only this program's children.
- Reuse one port. Kill the old listener before binding it again; never step to the
  next free port to avoid stopping the previous server.
- Prefer no server at all. `rsvg-convert`, or a `file://` path, answers most
  rendering questions. Start HTTP only when the question genuinely needs HTTP
  semantics, such as the published root or relative asset paths.
- Wrap anything that can block forever in `timeout`. A script reading from stdin
  that ends up waiting on a socket never returns, and orphans when its shell dies.
- Before answering, confirm nothing survived: `ps -eo pid,ppid,etime,command` and
  `lsof -nP -iTCP -sTCP:LISTEN`, and check for children reparented to pid 1.

## Stop Conditions

Stopping means asking in the reply, in prose, to the standard in `AGENTS.md`: what raised the question, what the answer changes, where the evidence is, what each way costs, and which is recommended. Never reach for a structured-question tool, a multiple-choice popup, or a confirmation dialog to do it — those fit a label per option and leave the user choosing between conclusions they cannot check. Settle anything a command or the code can answer before asking at all.

Stop and ask the user when:

- A version number would change, or anything would be published, and the user did not ask for it in that message. Finishing the work is not the authorisation, and neither is a release the user asked for earlier.
- Requirements conflict.
- A secret or private data appears in files or logs.
- A structural decision would materially change the committed plan.
- Verification fails in a way that cannot be resolved locally without guessing.

## Output Expectations

Keep final summaries short. Report what changed, how it was verified, and any remaining risk. Anything the user has to decide goes in the same reply as prose, never as a control they click. Do not mention private reference sources in user-facing summaries unless the user explicitly asks outside repository content.
