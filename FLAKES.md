# FLAKES

This file documents tests marked `#[ignore]` and why they remain excluded from automated runs.

| Test | Reason |
| ---- | ------ |
| `tests/packaging.rs::service_unit_matches_spec` | Service unit packaging checks require systemd packaging context and are not essential for core sync behavior. |
| `tests/cli.rs::default_umask_masks_permissions` | Umask handling under review. |
| `tests/no_implied_dirs.rs::preserves_symlinked_implied_dirs` | Symlinked implied directory behavior unfinished. |
