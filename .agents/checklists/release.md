# Release Checklist

Nothing below may start until the first two hold. They are not formalities: a
crates.io upload cannot be undone, and a version number is a promise to users.

- The user asked for a release, in the message being answered. Work being finished is not a request. A release the user asked for earlier does not cover this one.
- The user chose the version number. It is not derived from the diff, from semver, or from what the change "clearly is".
- CI is green.
- Coverage meets the current threshold.
- Release binaries are built for supported platforms.
- Checksums are generated and verified.
- Action install path is smoke-tested.
- CLI dashboard generation is smoke-tested.
- Server image or deployment artifact is smoke-tested when changed.
- Documentation matches released behavior.
- Private forbidden-reference scan passed.
- A crates.io upload is separately asked for. It is irreversible: a version can only be yanked, stays downloadable, and its number can never be reused.
