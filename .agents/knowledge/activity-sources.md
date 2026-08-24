# Where local activity can be read from

Notes on instrumenting coding tools, and the order to prefer. Sizes and schemas
below were measured on one development machine on 2026-08-24; treat them as
evidence that an approach is viable, not as constants.

## Prefer in this order

1. **The tool's own hooks.** Codex, Claude Code, Kimi Code, DeepSeek Harness and
   OpenCode all ship documented hook systems that hand over a JSON payload with
   session id, cwd, tool name and tool input. Codex's own documentation
   disclaims the stability of its transcript format while documenting its hooks,
   which settles the question of which is the supported interface.
2. **OpenTelemetry, where it exists.** Claude Code exports
   `claude_code.lines_of_code.count` and `claude_code.active_time.total` behind
   `CLAUDE_CODE_ENABLE_TELEMETRY=1`. Those are the two figures this project
   wants, already computed, over a protocol that needs no per-tool parsing.
3. **A tool's own index, for backfill.** Cheap, and answers "what changed" in
   one statement.
4. **Reading transcripts, last.** Undocumented shapes that drift, and the only
   option for work that happened before a hook was installed.

## Why transcripts cannot be the primary path

Measured sizes of the session stores on one machine:

| Store | Size |
| --- | --- |
| `~/.codex/sessions` | 12 GB |
| `~/.cursor/projects` | 1.3 GB |
| `~/.kimi/sessions` | 1.2 GB |
| `~/.kimi-code/sessions` | 149 MB |
| `~/.claude/projects` | 25 MB |

Roughly 15 GB in total, which rules out re-reading on every collection. Anything
that touches these must keep a resume position and stream, never load a file
whole. This is the one place in the collector where incremental reading is not an
optimisation but a requirement.

## Codex has a usable index

`~/.codex/state_5.sqlite`, table `threads`, 571 rows, 38 columns. The useful
ones: `rollout_path`, `cwd`, `git_branch`, `git_sha`, `model`, `tokens_used`,
`cli_version`, `created_at_ms`, `updated_at_ms`.

`updated_at_ms` was populated on every row, with no nulls, so
`WHERE updated_at_ms > <cursor>` is a sound incremental read and avoids walking
12 GB. Both second and millisecond columns exist and agreed with each other.

The file name carries a schema number, so discover it by glob (`state_*.sqlite`)
rather than hard-coding, and expect a migration to rename it.

## Traps worth knowing before writing any of this

- **Codex gates hooks behind a trust hash.** `~/.codex/config.toml` records
  `trusted_hash` per hook definition, and changing the hook's command string
  invalidates it and silently stops the hook until the user re-approves through
  `/hooks`. So the registered command must be a stable path to a wrapper script,
  with all churn inside the script. A collector that auto-updates its hook
  command would break itself on every release.
- **Shell hooks cannot see an agent's shell commands.** A Bash tool call spawns a
  non-interactive shell that sources neither `.zshrc` nor `.bashrc`, so `preexec`
  never fires. Agent shell work has to come from the agent's own hooks. Getting
  this wrong means instrumenting interactive work twice and agent work not at
  all.
- **Kimi Code emits `SessionHeartbeat` every 60 seconds** with an `uptime_ms`
  payload, and only runs the timer when the event is configured. That is a
  presence signal that needs no inference from tool calls.
- **`Stop` does not fire when a user interrupts Kimi Code**; `Interrupt` does.
  Closing sessions only on `Stop` leaks a session per interrupt.
- **A document change is not evidence a person did the work.** This one has
  already cost us a bug: the editor plugin subscribed to document changes, which
  fire for an agent's edits too, so agent work was landing in the measure of its
  author being at the editor. Caret movement is the signal that means a person.

## Privacy boundary these sources force

Hook payloads are far richer than what should be kept, so the discarding has to
happen at the point of capture:

- A `PostToolUse` payload for a write carries **the entire file content** in
  `tool_input.content`, and Codex's `patch_apply_end` carries full contents and
  unified diffs. Take the path, or count the lines, and drop the rest without
  buffering or logging it.
- **Do not store command text.** `argv[0]`, cwd, duration and exit code support
  "forty minutes running tests in this project" without ever holding a secret,
  and shell history routinely contains inline credentials, bearer tokens and
  database URLs with passwords. Full text capture, if ever offered, is opt-in.
- Never capture stdout or stderr; worse secret density than the command line and
  almost no value for measuring time.
- Honour what the user already told their shell: `HISTIGNORE`, and the
  leading-space convention of `HISTCONTROL=ignorespace`.
- Codex spills hook output over roughly 2,500 tokens to a temporary file, so a
  hook's stdout should stay empty.

## Not verified

- Where Kimi Code CLI writes per-turn message content. The session directory
  sampled held only `state.json` and an `agents/` subtree.
- Whether Gemini CLI has any hook system. None was found, which is a failure to
  find rather than evidence of absence.
- Crush's session database location; its own documentation contradicts itself.
- Whether the `threads` schema above matches public Codex releases. It was read
  from a build reporting a version ahead of the open-source ones.
- "deepseek-harness" is a real CLI (`dsh`); "harmony" is an unrelated token
  format for `gpt-oss` models, with nothing to instrument. Do not conflate them.
