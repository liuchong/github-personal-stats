# Deployment Knowledge

## Action

The Action path downloads a prebuilt release binary for the runner platform, verifies it, and runs it. It must not compile Rust in the consuming repository.

## CLI

The CLI is the stable local interface and should share behavior with the Action and HTTP server.

## Server

The HTTP server should expose the same core renderer through request parameters and should be deployable on common container and cloud platforms.

## Release

Release assets must include platform binaries and checksums. Smoke tests must verify installation and at least one dashboard generation path.
