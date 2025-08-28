# Project Gaps

This document tracks outstanding gaps in `rsync-rs` compared to the reference `rsync` implementation. Update this file as features are implemented.

## Missing rsync behaviors
- Checksum-based skipping via `-c/--checksum` is parsed but still unimplemented.
- Deletion flags other than `--delete` are not supported.
- Many command-line options remain absent or lack parity; see `docs/feature_matrix.md` for the full matrix.

## Unreachable code
- No `unreachable!` or similar markers were found in the current codebase, but manual audit may reveal latent issues.

## TODOs
- No `TODO` markers are present in the repository at this time.

## Test coverage gaps
- `tests/golden/cli_parity/delete.sh` skips its parity check when `rsync-rs --delete` fails, leaving deletion behavior partially untested.
- `tests/remote_remote.rs` contains an ignored test (`remote_to_remote_pipes_data`) for remote-to-remote transfers.
- Many CLI options listed in `docs/feature_matrix.md` have no associated tests.

## Continuous integration deficiencies
- Coverage is collected on Linux and Windows using `cargo-llvm-cov`, but no thresholds are enforced.
- Nightly jobs fuzz all targets for longer runs, yet pull requests still rely on brief smoke tests.
