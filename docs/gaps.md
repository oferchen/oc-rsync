# Project Gaps

This document tracks outstanding gaps in `oc-rsync` compared to the reference `rsync` implementation. Update this file as features are implemented. For a per-option overview see [feature_matrix.md](feature_matrix.md) and for high-level parity notes see [differences.md](differences.md).

## Recently addressed gaps
- Partial transfer resumption now reuses `.partial` files and retransfers only missing blocks.

## Protocol gaps
- `--block-size` — behavior differs from upstream. [tests/block_size.rs](../tests/block_size.rs) [feature_matrix](feature_matrix.md#L17)
- `--bwlimit` — rate limiting semantics differ. [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs) [feature_matrix](feature_matrix.md#L19)
 - `--contimeout` — connection timeout handling incomplete. [tests/timeout.rs](../tests/timeout.rs) [feature_matrix](feature_matrix.md#L32)
 - `--secluded-args` — not implemented. [feature_matrix](feature_matrix.md#L132) ([TODO](#testing-gaps))
 - `--server` — handshake lacks full parity. [tests/server.rs](../tests/server.rs) [feature_matrix](feature_matrix.md#L134)
- `--sockopts` — not implemented. [feature_matrix](feature_matrix.md#L137) ([TODO](#testing-gaps))
- `--timeout` — timeout semantics differ. [tests/timeout.rs](../tests/timeout.rs) [feature_matrix](feature_matrix.md#L147)

## Metadata gaps
- `--acls` — ACL support requires optional feature and lacks parity. [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) [tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) [feature_matrix](feature_matrix.md#L9)
- `--atimes` — access time preservation incomplete. [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) [feature_matrix](feature_matrix.md#L14)
 - `--copy-devices` — not implemented. [feature_matrix](feature_matrix.md#L35) ([TODO](#testing-gaps))
 - `--devices` — device file handling lacks parity. [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) [feature_matrix](feature_matrix.md#L52)
 - `--groupmap` — numeric gid mapping only; group names unsupported and requires root or CAP_CHOWN. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L74)
 - `--hard-links` — hard link tracking incomplete. [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) [feature_matrix](feature_matrix.md#L69)
 - `--keep-dirlinks` — not implemented. [feature_matrix](feature_matrix.md#L84) ([TODO](#testing-gaps))
 - `--links` — symlink handling lacks parity. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L86)
 - `--owner` — ownership restoration lacks parity. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L113)
 - `--perms` — permission preservation incomplete. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L117)
 - `--usermap` — numeric uid mapping only; user names unsupported and requires root or CAP_CHOWN. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L163)
 - `--xattrs` — extended attribute support requires optional feature and lacks parity. [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) [tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) [feature_matrix](feature_matrix.md#L157)

## Filter gaps
- `--exclude` — filter syntax coverage incomplete. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L56)
- `--exclude-from` — partial support for external lists. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L57)
- `--files-from` — partial support for list files. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L61)
- `--from0` — null-separated list handling incomplete. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L64)
- `--include` — filter syntax coverage incomplete. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L77)
- `--include-from` — partial support for external lists. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L78)

## Daemon gaps
- `--address` — binding to specific address lacks parity. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L10)
- `--daemon` — daemon mode incomplete. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L41)
- `--no-motd` — MOTD suppression lacks parity. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L101)
- `--password-file` — authentication semantics differ. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L116)
- `--port` — custom port handling incomplete. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L118)
- `--ipv4`/`--ipv6` — protocol selection lacks parity. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L81) [feature_matrix](feature_matrix.md#L82)
- `--secrets-file` — module authentication incomplete. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L133)
- `--timeout` — connection timeout semantics differ. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L147)

## Resume/partials gaps
- `--append`/`--append-verify` — remote resume semantics untested. [tests/resume.rs](../tests/resume.rs) [feature_matrix](feature_matrix.md#L11) [feature_matrix](feature_matrix.md#L12) ([TODO](#testing-gaps))
- `--partial-dir` — remote partial directory handling lacks parity. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L118) ([TODO](#testing-gaps))

## Deletion policy gaps
- `--delete-missing-args` — not implemented. [feature_matrix](feature_matrix.md#L51) ([TODO](#testing-gaps))
- `--ignore-errors` — not implemented. [feature_matrix](feature_matrix.md#L73) ([TODO](#testing-gaps))
- `--max-delete` — not implemented. [feature_matrix](feature_matrix.md#L91) ([TODO](#testing-gaps))
- `--remove-source-files` — not implemented. [feature_matrix](feature_matrix.md#L131) ([TODO](#testing-gaps))

## Progress & exit code gaps
- `--progress` — progress output differs from upstream. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L123)
- Exit code propagation across transports lacks coverage. [crates/protocol/tests/exit_codes.rs](../crates/protocol/tests/exit_codes.rs) [tests/partial_transfer_resume.sh](../tests/partial_transfer_resume.sh) ([TODO](#testing-gaps))

## Error propagation gaps
- Error forwarding between remote endpoints not thoroughly tested. [tests/server.rs](../tests/server.rs) [tests/remote_remote.rs](../tests/remote_remote.rs) ([TODO](#testing-gaps))

## Performance knob gaps
- `--max-alloc` — not implemented. [feature_matrix](feature_matrix.md#L90) ([TODO](#testing-gaps))
- `--preallocate` — not implemented. [feature_matrix](feature_matrix.md#L122) ([TODO](#testing-gaps))
- `--temp-dir` — cross-filesystem behavior differs. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L149)

## CI gaps
- CI covers only Linux builds; other platforms are cross-compiled without tests. [compatibility.md](compatibility.md#L11) [compatibility.md](compatibility.md#L13) ([TODO](#testing-gaps))
- Interop matrix lacks scenarios for resume and progress flags. [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh) ([TODO](#testing-gaps))

## Testing gaps
Many options above lack automated tests. Options marked with `[TODO](#testing-gaps)` need dedicated coverage in `tests/` before gaps can be closed.
