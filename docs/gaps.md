# Gap Analysis

This page enumerates known gaps between **oc-rsync** and upstream
`rsync`. Each item links to the relevant source code and test
coverage so progress can be tracked as features land.

## Protocol
- `--bwlimit` — rate limiting semantics differ. [transport/src/rate.rs](../crates/transport/src/rate.rs) · [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs)
- `--contimeout` — connection timeout handling incomplete. [cli/src/lib.rs](../crates/cli/src/lib.rs) · [tests/timeout.rs](../tests/timeout.rs)
- `--server` — handshake lacks full parity. [protocol/src/server.rs](../crates/protocol/src/server.rs) · [tests/server.rs](../tests/server.rs)
- `--timeout` — timeout semantics differ. [transport/src/lib.rs](../crates/transport/src/lib.rs) · [tests/timeout.rs](../tests/timeout.rs)

## Metadata
- `--acls` — ACL support requires optional feature and lacks parity. [meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs)
- `--atimes` — access time preservation incomplete. [meta/src/lib.rs](../crates/meta/src/lib.rs) · [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs)
- `--devices` — device file handling lacks parity. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)
- `--groupmap` — numeric gid mapping only; group names unsupported. [meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/cli.rs](../tests/cli.rs)
- `--hard-links` — hard link tracking incomplete. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)
- `--links` — symlink handling lacks parity. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--owner` — ownership restoration lacks parity. [meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/cli.rs](../tests/cli.rs)
- `--perms` — permission preservation incomplete. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--usermap` — numeric uid mapping only; user names unsupported. [meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/cli.rs](../tests/cli.rs)
- `--xattrs` — extended attribute support requires optional feature and lacks parity. [meta/src/unix.rs](../crates/meta/src/unix.rs) · [tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs)

## Filters
- `--exclude` — filter syntax coverage incomplete. [filters/src/lib.rs](../crates/filters/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--exclude-from` — partial support for external lists. [filters/src/lib.rs](../crates/filters/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--files-from` — partial support for list files. [filters/src/lib.rs](../crates/filters/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--from0` — null-separated list handling incomplete. [cli/src/lib.rs](../crates/cli/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--include` — filter syntax coverage incomplete. [filters/src/lib.rs](../crates/filters/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)
- `--include-from` — partial support for external lists. [filters/src/lib.rs](../crates/filters/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)

## Daemon
- `--address` — binding to specific address lacks parity. [daemon/src/lib.rs](../crates/daemon/src/lib.rs) · [tests/daemon.rs](../tests/daemon.rs)
- `--daemon` — daemon mode incomplete. [daemon/src/lib.rs](../crates/daemon/src/lib.rs) · [tests/daemon.rs](../tests/daemon.rs)
- `--no-motd` — MOTD suppression lacks parity. [daemon/src/lib.rs](../crates/daemon/src/lib.rs) · [tests/daemon.rs](../tests/daemon.rs)
- `--password-file` — authentication semantics differ. [daemon/src/lib.rs](../crates/daemon/src/lib.rs) · [tests/daemon.rs](../tests/daemon.rs)
- `--port` — custom port handling incomplete. [daemon/src/lib.rs](../crates/daemon/src/lib.rs) · [tests/daemon.rs](../tests/daemon.rs)
- `--ipv4`/`--ipv6` — protocol selection lacks parity. [daemon/src/lib.rs](../crates/daemon/src/lib.rs) · [tests/daemon.rs](../tests/daemon.rs)
- `--secrets-file` — module authentication incomplete. [daemon/src/lib.rs](../crates/daemon/src/lib.rs) · [tests/daemon.rs](../tests/daemon.rs)
- `--timeout` — connection timeout semantics differ. [daemon/src/lib.rs](../crates/daemon/src/lib.rs) · [tests/daemon.rs](../tests/daemon.rs)

## Resume/Partials
- `--append` / `--append-verify` — remote resume semantics untested. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/resume.rs](../tests/resume.rs)
- `--partial-dir` — remote partial directory handling lacks parity. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/cli.rs](../tests/cli.rs) *(needs dedicated test)*

## Progress & Exit Codes
- `--progress` — progress output differs from upstream and current progress tests fail to compile. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/cli.rs#L332](../tests/cli.rs#L332)
- Exit code propagation across transports lacks coverage. [protocol/src/lib.rs](../crates/protocol/src/lib.rs) · [crates/protocol/tests/exit_codes.rs](../crates/protocol/tests/exit_codes.rs)

## Error Propagation
- Forwarding errors between remote endpoints is under-tested. [protocol/src/demux.rs](../crates/protocol/src/demux.rs) · [tests/remote_remote.rs](../tests/remote_remote.rs)

## Performance Knobs
- `--temp-dir` — cross-filesystem behavior differs. [engine/src/lib.rs](../crates/engine/src/lib.rs) · [tests/cli.rs](../tests/cli.rs)

## CI
- CI runs only on Linux; other platforms are cross-compiled without tests. [compatibility.md](compatibility.md) · [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh)
- Interop matrix lacks scenarios for resume and progress flags. [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh)

## Testing
Many options above lack dedicated coverage; expand tests under
[tests/](../tests) to close remaining gaps.

