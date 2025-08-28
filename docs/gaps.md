# Project Gaps

This document tracks outstanding gaps in `rsync-rs` compared to the reference `rsync` implementation. Update this file as features are implemented.

## Missing rsync behaviors
- The `--server` handshake is not implemented, leaving server-mode negotiation incomplete; see the `--protocol` entry in `docs/feature_matrix.md`.
- Remote shell (`--rsh`) handling is incomplete (see the `--rsh` row in `docs/feature_matrix.md`).
- Daemon mode achieves only partial feature parity; `--password-file` and `--secrets-file` remain incomplete (see `docs/feature_matrix.md`).
- Filter rules are incomplete and do not yet match `rsync`'s full include/exclude syntax.
- Compression negotiation between peers is unimplemented.
- Many command-line options remain absent or lack parity; see `docs/feature_matrix.md` for the full matrix.

## Unreachable code
- No `unreachable!` or similar markers were found in the current codebase, but manual audit may reveal latent issues.

## TODOs
- No `TODO` markers are present in the repository at this time.

## Test coverage gaps
- `tests/golden/cli_parity/delete.sh` skips parity checks for deletion flags that are unimplemented or fail, which can hide gaps in deletion behavior.
- `tests/remote_remote.rs` exercises remote-to-remote transfers but only covers a basic pipe scenario; broader coverage for `--rsh` and related flows is missing (see `docs/feature_matrix.md` `--rsh`).
- Filter and compression negotiation lack dedicated tests; see `docs/feature_matrix.md` entries for `--filter`, `--compress`, and `--compress-level`.
- Many CLI options listed in `docs/feature_matrix.md` have no associated tests.

## Continuous integration deficiencies
- Coverage is collected on Linux and Windows using `cargo-llvm-cov` with `--fail-under-lines 80` and `--fail-under-functions 80` thresholds.
  Raise these thresholds as the test suite stabilizes.
- Nightly jobs fuzz all targets for longer runs, yet pull requests still rely on brief smoke tests.
- Remote-to-remote transfers, filter rules, and compression negotiation lack dedicated CI coverage.
