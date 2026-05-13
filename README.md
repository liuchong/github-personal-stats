# GitHub Stats

GitHub Stats is a Rust workspace for generating profile metrics as a unified SVG dashboard. The default output is one dashboard image so README layout is controlled by the renderer instead of by multiple images and HTML alignment rules.

## Goals

- Generate a single dashboard SVG by default.
- Support individual cards for stats, languages, streaks, repositories, gists, status, and coding activity.
- Provide a local CLI, release-binary GitHub Action, and HTTP server.
- Keep rendering dimensions explicit and configurable.
- Keep tests dense enough to protect aggregation, rendering, and deployment behavior.

## Repository Layout

- `crates/core`: shared data model, aggregation, rendering, and configuration.
- `crates/cli`: command-line interface.
- `crates/server`: HTTP interface.
- `.agents`: durable AI development memory and process files.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Coverage is enforced in CI with `cargo llvm-cov` once product logic grows beyond the foundation skeleton.

## License

This project is licensed under 1PL. See `LICENSE`.
