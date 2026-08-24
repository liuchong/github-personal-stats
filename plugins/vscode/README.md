# GitHub Personal Stats for VS Code

Reports what you are working on to the local daemon, so editor time can be
measured next to agent time. One extension covers every editor built on the VS
Code API, which includes VS Code, Cursor, VSCodium and Windsurf.

## What leaves your machine

Nothing. The extension talks only to `127.0.0.1`, and the daemon it talks to
writes to a file on the same machine.

What leaves the *editor process* is deliberately thin: a timestamp, the local
date, a file extension, and whether the file was being changed rather than read.
There is no path, no project name, no repository and no file content in a pulse.
The extension is the only part that ever sees a path, and it keeps it — which is
also why the extension decides what a file's kind is, rather than sending
something the daemon would have to inspect.

Files that are not on disk are ignored: output panels, diff views, settings
editors and untitled buffers are not work on a project, and counting them would
inflate the record.

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
| `githubPersonalStats.pulseSeconds` | `30` | How often continuous work turns into a pulse. |

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

It simulates edits to two files, then flushes. The journal under
`/tmp/state/pulses/` should hold one line per pulse, with extensions and no paths.
