# Project Gaps

This document tracks outstanding gaps in `rsync-rs` compared to the reference `rsync` implementation. Update this file as features are implemented.

## Recently addressed gaps
- Partial transfer resumption now reuses `.partial` files and retransfers only missing blocks.

## Missing rsync behaviors

### Protocol gaps
- Compression support includes zlib and zstd, with optional LZ4 available when the `lz4` feature is enabled.

### Metadata gaps
- File time preservation is incomplete; creation times (`--crtimes`) may not be supported on all platforms.
- Enhanced metadata such as permissions, owners, and groups lack full parity with GNU rsync.

### Filter gaps
- Filter rules cover basic include/exclude patterns but still fall short of `rsync`'s full syntax, lacking advanced rule modifiers and merge directives.

### Daemon gaps
- Many command-line options remain absent or lack parity; see `docs/feature_matrix.md` for the full matrix.

## Unreachable code
- No `unreachable!` or similar markers were found in the current codebase, but manual audit may reveal latent issues.

## TODOs
- No `TODO` markers are present in the repository at this time.

## Test coverage gaps
 - Many CLI options listed in `docs/feature_matrix.md` have no associated tests.

## Continuous integration deficiencies
- Coverage is collected on Linux and Windows using `cargo-llvm-cov` with `--fail-under-lines 80` and `--fail-under-functions 80` thresholds.
  Raise these thresholds as the test suite stabilizes.
- Nightly jobs fuzz all targets for longer runs, yet pull requests still rely on brief smoke tests.
