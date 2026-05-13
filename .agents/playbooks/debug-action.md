# Playbook: Debug Action

1. Reproduce with the smallest workflow or local command.
2. Identify whether failure is install, checksum, permissions, configuration, CLI execution, or git writeback.
3. Keep consuming workflows free of Rust compilation.
4. Add a smoke test when the failure mode can recur.
5. Update deployment knowledge if the install path changes.
6. Run verification and forbidden-reference scan.
