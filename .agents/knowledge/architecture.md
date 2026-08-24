# Architecture Knowledge

## Boundaries

- Interface layers parse user input and write output.
- Data clients fetch remote data and classify errors.
- Aggregators transform fetched data into stable card data.
- Renderers transform card data into SVG, text, JSON, or PNG output.
- Deployment layers package and expose the same CLI/core behavior.

## Default Shape

The default output is a single dashboard SVG so layout is controlled inside the renderer instead of relying on README HTML behavior.

## Aggregation Shape

Core aggregation produces `CardData` values. Dashboard data is composed from the same stats, language, and streak summaries used by individual card outputs, so renderer differences do not fork business rules.

## Activity Storage Backends

Local activity is collected on the machine that has the records and rendered somewhere else, so a snapshot has to travel. `sink::Sink` is that boundary, and there are three backends behind it. Which one an installation uses is configuration, not a code path chosen at build time.

- **File.** Writes one snapshot to one path. For a machine that renders its own cards, or that moves the file by some other means. This is the default.
- **Git.** Writes `snapshots/<machine>.json` into a git repository, commits, and pushes. This is the backend for the serverless case: a private repository is storage, and CI reads it to render. It is deliberately not a GitHub backend. Any git remote the collector can reach will do, public internet or not, and nothing in the sink knows the name of a host. Shelling out to `git` rather than calling one host's API is what buys that.
- **HTTP.** For a hosted service collecting from many machines. The trait is the seam; the implementation is left out rather than guessed at, because the protocol and the notion of identity belong to whoever runs such a service.

Rules that hold across backends:

- One file per machine. Machines never write the same path, so a shared repository has nothing to merge, and the reader adds the files up itself. `merge_snapshots` keys on machine and date for exactly this reason.
- Publication is compared on the record, not the bytes. Every collection moves `collected_at`, so a byte comparison would make a daemon on a timer commit every time it wakes. `ActivitySnapshot::records_the_same_as` is what a history-keeping sink asks.
- The git checkout is storage the app owns. It lives in the runtime state directory, is cloned when absent, is brought up to date before each commit, and can be deleted at any time. It is not somewhere a user works.
- An unreachable remote is not a lost collection. The commit stays local and the next run pushes it.

## Durability Rule

When a boundary changes, update this file and record the decision in `.agents/decisions.md`.
