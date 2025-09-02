# Gap Analysis

This page enumerates known gaps between **oc-rsync** and upstream
`rsync`. Each item links to the relevant source code and test
coverage so progress can be tracked as features land.

## Protocol
No known gaps.

## Compression
No known gaps.

## Messages
Message handling lacks full parity; only a subset of upstream message types is implemented. [protocol/src/lib.rs](../crates/protocol/src/lib.rs) · [crates/protocol/tests/protocol.rs](../crates/protocol/tests/protocol.rs)

## Exit Codes
No known gaps. Exit codes map to upstream values. [protocol/src/lib.rs](../crates/protocol/src/lib.rs) · [crates/protocol/tests/exit_codes.rs](../crates/protocol/tests/exit_codes.rs)

## Metadata
- `--archive` — composite flag; underlying `--owner`, `--group`, and `--perms` gaps apply. [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) · [tests/archive.rs](../tests/archive.rs)
- `--acls` — ACL support requires optional feature and lacks parity. [meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) · [tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs)
- `--hard-links` — hard link tracking incomplete. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)
    - Resolved: multi-link groups with partially existing destinations now replay correctly. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)
- `--owner` — ownership restoration lacks parity. [meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/cli.rs](../tests/cli.rs)
- `--perms` — permission preservation incomplete. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)

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
No known gaps.

## CI
- CI runs only on Linux; other platforms are cross-compiled without tests. [compatibility.md](compatibility.md) · [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh)
Interop matrix scenarios are defined in [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh) and must stay in sync with this documentation:

  - `base`
  - `delete`
  - `delete_before`
  - `delete_during`
  - `delete_after`
  - `compress`
  - `rsh`
  - `drop_connection`
  - `vanished`
  - `remote_remote`
  - `append`
  - `append_verify`
  - `partial`
  - `inplace`
  - `resume`
  - `progress`
  - `resume_progress`
  - `progress2`
  - `resume_progress2`

## Testing
Many options above lack dedicated coverage; expand tests under
[tests/](../tests) to close remaining gaps.

