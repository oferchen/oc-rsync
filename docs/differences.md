# Differences from rsync

oc-rsync implements the standard rsync protocol version 32 and also defines a
private protocol 73 extension used only when both peers are oc-rsync.

See [gaps.md](gaps.md) and [feature_matrix.md](feature_matrix.md) for any remaining parity notes.

## CLI deviations

While maintaining parity with upstream `rsync`, `oc-rsync` extends the command
line with a few additional flags and behaviors:

- `--config` loads settings from a TOML file.
  See [feature matrix](feature_matrix.md#--config) and
  [tests/cli.rs](../tests/cli.rs).
- `--log-format=json` emits structured logs.
  See [tests/cli.rs](../tests/cli.rs).
- Flags in the `--modern*` namespace enable optional enhancements.
  See [feature matrix](feature_matrix.md#--modern).

Document any future CLI differences here as they arise.

## Exit codes

`oc-rsync` mirrors `rsync`'s exit code semantics and forwards unknown values
across transports. Behavior is validated in
[crates/protocol/tests/exit_codes.rs](../crates/protocol/tests/exit_codes.rs)
and end-to-end scenarios like
[tests/partial_transfer_resume.sh](../tests/partial_transfer_resume.sh).

## Modern mode specifics

When `--modern` is negotiated between two `oc-rsync` peers (protocol 73), the
following features become available:

- BLAKE3 strong checksums via `--modern` or `--modern-hash`.
- zstd or lz4 compression via `--modern` or `--modern-compress`.
- Optional FastCDC chunking via `--modern-cdc`.

Coverage exists in [tests/interop/modern.rs](../tests/interop/modern.rs),
[tests/golden/cli_parity/modern_flags.sh](../tests/golden/cli_parity/modern_flags.sh),
and [tests/cdc.rs](../tests/cdc.rs). See
[feature matrix](feature_matrix.md#--modern),
[feature matrix](feature_matrix.md#--modern-compress),
[feature matrix](feature_matrix.md#--modern-hash), and
[feature matrix](feature_matrix.md#--modern-cdc) for implementation status.

## File list encoding

Paths in the file list are delta-encoded and use uid/gid lookup tables, matching
the behavior of upstream rsync.

## Manifest location

The content-defined chunking manifest now resides at `~/.oc-rsync/manifest`.
Previous versions stored it at `~/.rsync-rs/manifest`. To continue using an
existing manifest, move it to the new location.

Keep this document updated as new differences are introduced.
