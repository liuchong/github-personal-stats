# github-personal-stats-daemon

The local half of activity collection. It listens for pulses from editor plugins,
rebuilds the activity snapshot on a timer, and serves a panel showing what has
been collected on this machine.

It listens on the loopback address and refuses to start on any other, because it
holds a machine's whole activity history and accepts writes. Nothing about it
needs to be reachable from another host.

## Running it

```bash
github-personal-stats-daemon serve
```

That prints the address, the panel URL including the token, and where the
snapshot is written. On first run it mints a shared secret in the state directory,
readable only by its owner; plugins read the same file, so they need no
configuration beyond finding it.

```bash
github-personal-stats-daemon token   # where the secret lives, and what it is
github-personal-stats-daemon help    # every option and endpoint
```

## Two measures, kept apart

A day holds two kinds of time, and they are not interchangeable:

- **Editor time** comes from plugins reporting what is being worked on. It is what
  a conventional time tracker measures.
- **Agent time** comes from the editor's own record of code an agent generated. It
  is time in which code was actually changing.

They can differ by a factor of seven on the same day — a long session of reading
and hand editing is mostly editor time, and a long agent run is mostly agent time.
Both are computed by the same rule, so a difference between them is a difference
in what was observed rather than in how it was counted: a gap no longer than the
idle timeout counts as time, and a longer gap ends the session.

## Endpoints

| Method | Path | What it does |
| --- | --- | --- |
| `GET` | `/v1/health` | Whether the daemon is up. The only endpoint that needs no token. |
| `POST` | `/v1/pulses` | Report pulses. |
| `POST` | `/v1/collect` | Rebuild the snapshot now. |
| `GET` | `/v1/summary` | The current totals, as JSON. |
| `GET` | `/` | The panel, as a page. |

The token goes in an `Authorization: Bearer` header, which is what a plugin sends,
or in a `token` query parameter, which is the only way a browser opening the panel
can present it.

A pulse carries a timestamp, a local date, a file extension and whether the file
was being changed. It carries no path, no project name and no file content, so
neither the journal nor anything derived from it can disclose where you work:

```bash
curl -X POST http://127.0.0.1:7391/v1/pulses \
  -H "Authorization: Bearer $(cat "$XDG_STATE_HOME/github-personal-stats/token")" \
  -d '{"editor":"vscode","pulses":[{"at":1787576128,"day":"2026-08-24","ext":"rs","write":true}]}'
```

Pulses are appended to a journal, one file per day and one line per pulse, and
aggregation only ever reads it. Sending the same batch twice is harmless: two
pulses at the same second leave no gap between them, so they add no time and no
session.
