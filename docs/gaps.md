# Gap Analysis

This page enumerates known gaps between **oc-rsync** and upstream
`rsync`. Each item links to the relevant source code and test
coverage so progress can be tracked as features land.

## Protocol
No known gaps.

## Metadata
- `--acls` — ACL support requires optional feature and lacks parity. [meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs)
- `--groupmap` — numeric gid mapping only; group names unsupported. [meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/cli.rs](../tests/cli.rs)
- `--links` — symlink handling lacks parity. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--hard-links` — hard link tracking incomplete. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)
- `--owner` — ownership restoration lacks parity. [meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/cli.rs](../tests/cli.rs)
- `--perms` — permission preservation incomplete. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--xattrs` — extended attribute support requires optional feature and lacks parity. [meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs)
- `--usermap` — numeric uid mapping only; user names unsupported. [meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/cli.rs](../tests/cli.rs)

## Filters
- `--exclude` — filter syntax coverage incomplete. [filters/src/lib.rs](../crates/filters/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--exclude-from` — partial support for external lists. [filters/src/lib.rs](../crates/filters/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--files-from` — partial support for list files. [filters/src/lib.rs](../crates/filters/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--from0` — null-separated list handling incomplete. [cli/src/lib.rs](../crates/cli/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--include` — filter syntax coverage incomplete. [filters/src/lib.rs](../crates/filters/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--include-from` — partial support for external lists. [filters/src/lib.rs](../crates/filters/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)

## Daemon
- `--daemon` — daemon mode incomplete. [daemon/src/lib.rs](../crates/daemon/src/lib.rs) · [tests/daemon.rs](../tests/daemon.rs)

## Transfer Mechanics

No known gaps. `--force` (forced deletion of non-empty directories) is supported and covered by tests `force_removes_non_empty_dirs` and `force_removes_nested_non_empty_dirs` in [tests/cli.rs](../tests/cli.rs).

Filename charset conversion via `--iconv` is supported and exercised by [tests/cli.rs](../tests/cli.rs).

## Resume/Partials

No known gaps.

## Logging
No known gaps.

## Performance Knobs
- `--temp-dir` — cross-filesystem behavior differs. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)

## CI
- CI runs only on Linux; other platforms are cross-compiled without tests. [compatibility.md](compatibility.md) · [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh)
- Interop matrix lacks scenarios for resume and progress flags. [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh)

## Testing
Many options above lack dedicated coverage; expand tests under
[tests/](../tests) to close remaining gaps.

