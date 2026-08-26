# GitHub Personal Stats for Emacs

Reports that you are at the editor to the same local record everything else writes
to, so hours spent in Emacs are counted beside hours spent anywhere else.

## What leaves your machine

Nothing, unless you publish it. The mode writes to `127.0.0.1` or to a file on
this machine, and publishing to your data repository is the collector's job.

What leaves the *Emacs process* is deliberately thin: a timestamp, the local date,
and a file extension. There is no path, no project name, no buffer name, no
repository and no buffer content in a pulse. This mode is the only part that ever
sees a path, and it keeps it — which is also why the mode decides what a file's
kind is, rather than sending something the collector would have to inspect.

## What it measures

Time at the editor, bounded by an idle timeout. Not time you spent typing.

That distinction is the whole design. Measuring keystrokes in a buffer sounds more
precise and is in practice useless: a day spent directing an agent produces almost
none, because the prompt goes somewhere that is not a buffer and the edits come
back from something that is not you.

So presence is the signal, with one cutoff. While Emacs has focus and has been
given input within `github-personal-stats-idle-seconds` — ten minutes by default —
a pulse is filed every `github-personal-stats-pulse-seconds`. Each is filed under
the kind of file in the selected window; a shell, a magit buffer or a help window
still counts as time, filed under no language, because you were there either way.

The cutoff is where this differs from the VS Code extension, which counts a
focused window whether or not anything is happening in it. Emacs is habitually
left in front of you for days, and counting that would report sleep as work.
Reading, scrolling and directing an agent all produce input, so ten minutes of
grace costs nothing real.

Time agents spent changing code is a **separate measure from a separate source**,
so an agent working while you are away is counted there and not here. The two
overlap by design and are never added together.

## Setting it up

```elisp
(add-to-list 'load-path "/path/to/github-personal-stats/plugins/emacs")
(require 'github-personal-stats)
(github-personal-stats-mode 1)
```

With `use-package`:

```elisp
(use-package github-personal-stats
  :load-path "/path/to/github-personal-stats/plugins/emacs"
  :config (github-personal-stats-mode 1))
```

There is nothing else to configure. If the daemon is running, pulses go to it and
show up in its panel immediately; if it is not, they go to the journal on disk that
the collector reads anyway. Either way the time is counted once.

`M-x github-personal-stats-status` says which of those is happening, and
`M-x github-personal-stats-send-now` sends whatever is queued and reports where it
went.

The mode line shows ` stats` while reporting, ` stats~` when it last fell back to
the journal, and ` stats!` when pulses are piling up rather than going anywhere.

## Where pulses go

| `github-personal-stats-sink` | Behaviour |
| --- | --- |
| `auto` (default) | Post to the daemon; write to the journal when it cannot be reached. |
| `daemon` | Post to the daemon only, queueing in memory until it answers. Nothing is written to disk, so a daemon that never answers costs those pulses when Emacs closes. |
| `journal` | Append to the journal only. No daemon, no port, no token. |

`journal` is the sink for a machine that runs Emacs but no daemon — a server you
ssh into, for instance. The journal is the daemon's own append-only file, so
whenever the collector next runs there it reads these pulses like any others:

```bash
github-personal-stats-collect --sink git --repo ~/.local/state/github-personal-stats/storage
```

The mode does not compute hours, decide what a language is, or publish anything.
Pulses are moments. Turning moments into sessions, and sessions into a published
record, happens once for every source rather than once per editor — which is why
adding an editor cannot change what an hour means.

## Settings

| Variable | Default | What it does |
| --- | --- | --- |
| `github-personal-stats-sink` | `auto` | Where pulses go. |
| `github-personal-stats-daemon-url` | `http://127.0.0.1:7391` | Where the daemon is listening. |
| `github-personal-stats-state-directory` | `nil` | Where the token and journal live. `nil` means the XDG state directory. |
| `github-personal-stats-pulse-seconds` | `30` | How often presence becomes a pulse. |
| `github-personal-stats-flush-seconds` | `60` | How often queued pulses are sent. |
| `github-personal-stats-idle-seconds` | `600` | How long without input still counts as being here. |
| `github-personal-stats-max-queued` | `2000` | How many unsent pulses to keep. |

Keep `pulse-seconds` well below the collector's idle timeout, which defaults to
five minutes. A gap longer than that timeout is treated as a break rather than as
time worked, so pulsing less often than the timeout would record no time at all.

## Tests

```bash
cd plugins/emacs
emacs -Q -batch -L . -l test/github-personal-stats-test.el -f ert-run-tests-batch-and-exit
```

What they check is mostly the boundary: the shape of a pulse, the shape of a
journal line, and what is never in either. The daemon rejects a batch it cannot
read and the collector refuses a journal it cannot parse, so a mode that gets the
shape wrong does not degrade quietly — it stops, and takes the day's other sources
with it.
