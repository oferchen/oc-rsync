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
Document any future CLI differences here as they arise.

## Exit codes

`oc-rsync` mirrors `rsync`'s exit code semantics and forwards unknown values
across transports. Behavior is validated in
[crates/protocol/tests/exit_codes.rs](../crates/protocol/tests/exit_codes.rs)
and end-to-end scenarios like
[tests/partial_transfer_resume.sh](../tests/partial_transfer_resume.sh).

## File list encoding

Paths in the file list are delta-encoded and use uid/gid lookup tables, matching
the behavior of upstream rsync.

Keep this document updated as new differences are introduced.
