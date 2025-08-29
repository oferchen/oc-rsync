# Project Gaps

This document tracks outstanding gaps in `rsync-rs` compared to the reference `rsync` implementation. Update this file as features are implemented.

## Missing rsync behaviors

### Protocol gaps
- Remote shell (`--rsh`) negotiation is incomplete, lacking full `rsh` command parsing and environment handshakes.
- Partial transfer resumption does not fully match `rsync` semantics; interrupted copies cannot reuse partially transferred data.
- Compression negotiation between peers is unimplemented.

### Metadata gaps
- File time preservation is incomplete; creation times (`--crtimes`) may not be supported on all platforms.
- Enhanced metadata such as permissions, owners, and groups lack full parity with GNU rsync.

### Filter gaps
- Filter rules are incomplete and do not yet match `rsync`'s full include/exclude syntax.
- Per-directory `.rsync-filter` handling and `-F` convenience flag semantics are unimplemented.

### Daemon gaps
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
