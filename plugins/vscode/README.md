# GitHub Personal Stats for VS Code

Reports that you are at the editor to the local daemon, so your time can be
measured next to the time agents spent changing code. One extension covers every
editor built on the VS Code API, which includes VS Code, Cursor, VSCodium and
Windsurf.

## What leaves your machine

Nothing. The extension talks only to `127.0.0.1`, and the daemon it talks to
writes to a file on the same machine.

What leaves the *editor process* is deliberately thin: a timestamp, the local
date, and a file extension. There is no path, no project name, no repository and
no file content in a pulse. The extension is the only part that ever sees a path,
and it keeps it — which is also why the extension decides what a file's kind is,
rather than sending something the daemon would have to inspect.

## What it measures

Time this window had focus. Not time you spent typing.

That distinction is the whole design. Asking the document API — saves, tab
switches, caret movements — sounds more precise and is in practice useless: a day
spent directing an agent touches none of those, because the prompt goes into a
panel that is not a document and the edits come back from something that is not
you. Measured over thirty-seven hours of real work, that approach reported
nothing whatsoever.

So a pulse is sent when this window takes focus and every `pulseSeconds` while it
keeps it, whatever is or is not being typed. Each one is filed under the kind of
file open at the time; a window showing an output panel or a settings page still
counts as time, filed under no language, because you were there either way.

The honest limitation: a window left focused while you walk away is counted until
it loses focus. The daemon's idle timeout bounds how far that can run, and it
cannot detect it. Reporting a little too much for a coffee break is a smaller
error than reporting nothing for a working day.

Time agents spent changing code is a **separate measure from a separate source**,
so an agent working while you are away is counted there and not here. The two
overlap by design and are never added together.

## Setting it up

Start the daemon first, because the extension authenticates with a token the
daemon mints on first run:

```bash
github-personal-stats-daemon serve
```

Then build and install the extension:

```bash
cd plugins/vscode
npm install
npm run compile
npm run package        # needs @vscode/vsce
code --install-extension github-personal-stats-vscode.vsix
```

In Cursor, use `cursor --install-extension` instead. The status bar shows a pulse
icon while reporting, a crossed circle when it cannot find the daemon's token, and
a count when pulses are waiting to be delivered. Clicking it sends what is queued
and reports where it is sending to.

## Settings

| Setting | Default | What it does |
| --- | --- | --- |
| `githubPersonalStats.enabled` | `true` | Whether to report at all. |
| `githubPersonalStats.daemonUrl` | `http://127.0.0.1:7391` | Where the daemon is listening. |
| `githubPersonalStats.statePath` | `""` | Where the daemon keeps its token. Empty means the XDG state directory. |
| `githubPersonalStats.pulseSeconds` | `30` | How often a focused window turns into a pulse. |

Keep `pulseSeconds` well below the daemon's idle timeout, which defaults to five
minutes. A gap longer than that timeout is treated as a break rather than as time
worked, so pulsing less often than the timeout would record no time at all.

If the daemon is not running, pulses queue in memory and go out when it comes
back, so restarting it does not cost the morning's work. The queue is bounded and
drops the oldest pulses first.

## Checking it works without installing it

`test/harness.cjs` drives the compiled extension with a stand-in for the editor
API, which is the only way to exercise the reporting path without an editor:

```bash
npm run compile
github-personal-stats-daemon serve --addr 127.0.0.1:7393 --state /tmp/state &
node test/harness.cjs /tmp/state http://127.0.0.1:7393
```

It focuses a window, lets it beat, blurs it, and flushes. The journal under
`/tmp/state/pulses/` should hold one line per pulse, with extensions and no paths,
and nothing at all from the time the window was in the background.
