# Differences from rsync

oc-rsync diverges from upstream rsync 3.4.x in the following areas:

- POSIX ACL handling requires the optional `acl` feature and does not yet match upstream semantics. [crates/meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) · [tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs)

Parity gaps and unsupported options are tracked in [gaps.md](gaps.md) and [feature_matrix.md](feature_matrix.md).

Keep this document updated as new differences are introduced.
