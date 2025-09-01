# Differences from rsync

oc-rsync implements the standard rsync protocol version 32.

See [gaps.md](gaps.md) and [feature_matrix.md](feature_matrix.md) for any remaining parity notes.

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
