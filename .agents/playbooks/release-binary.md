# Playbook: Release Binary

0. Confirm the user asked for this release and gave the version number, in the message being answered. If either is missing, stop here and ask. Do not choose a number and do not read a finished feature as permission.
1. Confirm version and changelog source.
2. Build supported platform binaries.
3. Generate checksums.
4. Smoke-test binary execution.
5. Smoke-test Action download and checksum verification.
6. Publish release assets.
7. Verify documentation points to the release flow.
8. Run forbidden-reference scan before publishing notes.
