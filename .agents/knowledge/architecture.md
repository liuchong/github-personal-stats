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

- **File.** Writes the record under one directory. For a machine that renders its own cards, or that moves the files by some other means. This is the default.
- **Git.** Commits the record into a git repository and pushes. This is the backend for the serverless case: a private repository is storage, and CI reads it to render. It is deliberately not a GitHub backend. Any git remote the collector can reach will do, public internet or not, and nothing in the sink knows the name of a host. Shelling out to `git` rather than calling one host's API is what buys that.
- **HTTP.** For a hosted service collecting from many machines. The trait is the seam; the implementation is left out rather than guessed at, because the protocol and the notion of identity belong to whoever runs such a service.

A sink decides where the root is and what to do after writing. It does not decide the shape; `records::publish` does, so every backend accumulates identically.

## A Collection Is Not The Record

This is the distinction the whole activity side turns on, and getting it wrong loses data silently.

A **collection** is a reading of the local sources, and those sources forget. Cursor's `ai-code-tracking.db` holds roughly thirty days of `ai_code_hashes`, so a collection made today describes today well and describes April not at all: the old day reads as zero seconds, zero languages, zero generated lines. `collect()` deliberately does not open what has been published, so a reading can never shrink a record it never read.

The **record** is what accumulates on disk, and it outlives every source it was read from. `records::publish` merges a collection into it rather than replacing it.

The merge rule is `DayBucket::keep_fuller`, taking the larger of each field. It is not `absorb`, which sums, and which would inflate the record on a timer since every run re-reads the same days. Taking the larger is right because a past day holds a fixed amount of work and a reading of it can only be complete or cut short, never larger than the truth. Its cost: a genuine correction downwards cannot land on a published day, so a changed measurement rule needs the day files deleted.

## How The Record Is Laid Out

`core::store`. One directory per machine, one file per day inside it, plus `manifest.json` holding the day index, the incremental cursors, and a rollup of lifetime totals.

- A day file is written once and afterwards only replaced by a fuller reading of that same day, so accumulation is structural instead of a merge step someone has to remember. The single growing file this replaced had to be rewritten whole every run, which meant a bug in the writer could erase years.
- A run touches only the days it learned something about, so a reader can fetch a window rather than everything, and each commit says which day it recorded instead of showing the whole record rewritten.
- The rollup exists because splitting by day would otherwise cost a reader after a lifetime total everything, which is the figure the cards show. It is a cache: written from the day files, never the other way round, and recomputable with `roll_up`.
- A day file repeats its schema and machine rather than inferring them from the path, so a file copied or fetched over HTTP still says whose it is.

Rules that hold across backends:

- One directory per machine. Machines never write the same path, so a shared repository has nothing to merge, and the reader adds the days up itself.
- Publication is compared on content, file by file. A day whose bytes are unchanged is not rewritten, and a manifest is only written when it describes something new — `collected_at` moves on every collection, so writing it regardless would make a commit out of reading the clock.
- A day file that cannot be read stops the run. It may hold the only surviving copy of a day the sources have forgotten, and a collection made today cannot reconstruct it, so overwriting it would destroy it silently. This reverses the earlier behaviour of treating an unreadable published file as absent, which was safe only while the record could be rebuilt from source.
- The git checkout is storage the app owns. It lives in the runtime state directory, is cloned when absent, is brought up to date before each commit, and can be deleted at any time. It is not somewhere a user works.
- An unreachable remote is not a lost collection. The commit stays local and the next run pushes it.
- Anything showing what has been collected reads the record, through `Sink::root`, not a fresh collection. A panel built from a collection would show a month and call it everything.

## Durability Rule

When a boundary changes, update this file and record the decision in `.agents/decisions.md`.
