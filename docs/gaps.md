# Project Gaps

This document tracks outstanding gaps in `oc-rsync` compared to the reference `rsync` implementation. Update this file as features are implemented.

## Recently addressed gaps
- Partial transfer resumption now reuses `.partial` files and retransfers only missing blocks.

## Protocol gaps
- `--8-bit-output` — not implemented. [feature_matrix](feature_matrix.md#L8) ([TODO](#testing-gaps))
- `--block-size` — behavior differs from upstream. [tests/block_size.rs](../tests/block_size.rs) [feature_matrix](feature_matrix.md#L17)
- `--blocking-io` — not implemented. [feature_matrix](feature_matrix.md#L18) ([TODO](#testing-gaps))
- `--bwlimit` — rate limiting semantics differ. [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs) [feature_matrix](feature_matrix.md#L19)
- `--checksum-seed` — not implemented. [feature_matrix](feature_matrix.md#L23) ([TODO](#testing-gaps))
- `--contimeout` — connection timeout handling incomplete. [tests/timeout.rs](../tests/timeout.rs) [feature_matrix](feature_matrix.md#L32)
- `--early-input` — not implemented. [feature_matrix](feature_matrix.md#L55) ([TODO](#testing-gaps))
- `--protocol` — protocol version override unsupported. [feature_matrix](feature_matrix.md#L121) ([TODO](#testing-gaps))
- `--remote-option` — not implemented. [feature_matrix](feature_matrix.md#L127) ([TODO](#testing-gaps))
- `--secluded-args` — not implemented. [feature_matrix](feature_matrix.md#L132) ([TODO](#testing-gaps))
- `--server` — handshake lacks full parity. [tests/server.rs](../tests/server.rs) [feature_matrix](feature_matrix.md#L134)
- `--size-only` — not implemented. [feature_matrix](feature_matrix.md#L135) ([TODO](#testing-gaps))
- `--skip-compress` — not implemented. [feature_matrix](feature_matrix.md#L136) ([TODO](#testing-gaps))
- `--sockopts` — not implemented. [feature_matrix](feature_matrix.md#L137) ([TODO](#testing-gaps))
- `--timeout` — timeout semantics differ. [tests/timeout.rs](../tests/timeout.rs) [feature_matrix](feature_matrix.md#L147)
- `--whole-file` — not implemented. [feature_matrix](feature_matrix.md#L154) ([TODO](#testing-gaps))
- `--write-batch` — not implemented. [feature_matrix](feature_matrix.md#L155) ([TODO](#testing-gaps))
- `--write-devices` — not implemented. [feature_matrix](feature_matrix.md#L156) ([TODO](#testing-gaps))

## Metadata gaps
- `--acls` — ACL support requires optional feature and lacks parity. [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) [tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) [feature_matrix](feature_matrix.md#L9)
- `--atimes` — access time preservation incomplete. [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) [feature_matrix](feature_matrix.md#L14)
- `--chmod` — not implemented. [feature_matrix](feature_matrix.md#L24) ([TODO](#testing-gaps))
- `--chown` — not implemented. [feature_matrix](feature_matrix.md#L25) ([TODO](#testing-gaps))
- `--copy-devices` — not implemented. [feature_matrix](feature_matrix.md#L35) ([TODO](#testing-gaps))
- `--devices` — device file handling lacks parity. [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) [feature_matrix](feature_matrix.md#L52)
- `--executability` — not implemented. [feature_matrix](feature_matrix.md#L58) ([TODO](#testing-gaps))
- `--groupmap` — not implemented. [feature_matrix](feature_matrix.md#L68) ([TODO](#testing-gaps))
- `--hard-links` — hard link tracking incomplete. [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) [feature_matrix](feature_matrix.md#L69)
- `--itemize-changes` — not implemented. [feature_matrix](feature_matrix.md#L83) ([TODO](#testing-gaps))
- `--keep-dirlinks` — not implemented. [feature_matrix](feature_matrix.md#L84) ([TODO](#testing-gaps))
- `--links` — symlink handling lacks parity. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L86)
- `--omit-dir-times` — not implemented. [feature_matrix](feature_matrix.md#L106) ([TODO](#testing-gaps))
- `--omit-link-times` — not implemented. [feature_matrix](feature_matrix.md#L107) ([TODO](#testing-gaps))
- `--owner` — ownership restoration lacks parity. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L113)
- `--perms` — permission preservation incomplete. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L117)
- `--usermap` — not implemented. [feature_matrix](feature_matrix.md#L151) ([TODO](#testing-gaps))
- `--xattrs` — extended attribute support requires optional feature and lacks parity. [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) [tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) [feature_matrix](feature_matrix.md#L157)

## Filter gaps
- `--exclude` — filter syntax coverage incomplete. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L56)
- `--exclude-from` — partial support for external lists. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L57)
- `--files-from` — partial support for list files. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L61)
- `--from0` — null-separated list handling incomplete. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L64)
- `--include` — filter syntax coverage incomplete. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L77)
- `--include-from` — partial support for external lists. [tests/cli.rs](../tests/cli.rs) [feature_matrix](feature_matrix.md#L78)
- `--existing` — not implemented. [feature_matrix](feature_matrix.md#L59) ([TODO](#testing-gaps))
- `--ignore-existing` — not implemented. [feature_matrix](feature_matrix.md#L74) ([TODO](#testing-gaps))
- `--delete-missing-args` — not implemented. [feature_matrix](feature_matrix.md#L51) ([TODO](#testing-gaps))
- `--prune-empty-dirs` — not implemented. [feature_matrix](feature_matrix.md#L122) ([TODO](#testing-gaps))
- `--remove-source-files` — not implemented. [feature_matrix](feature_matrix.md#L128) ([TODO](#testing-gaps))

## Daemon gaps
- `--address` — binding to specific address lacks parity. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L10)
- `--daemon` — daemon mode incomplete. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L41)
- `--no-motd` — MOTD suppression lacks parity. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L101)
- `--password-file` — authentication semantics differ. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L116)
- `--port` — custom port handling incomplete. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L118)
- `--ipv4`/`--ipv6` — protocol selection lacks parity. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L81) [feature_matrix](feature_matrix.md#L82)
- `--secrets-file` — module authentication incomplete. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L133)
- `--timeout` — connection timeout semantics differ. [tests/daemon.rs](../tests/daemon.rs) [feature_matrix](feature_matrix.md#L147)

## Testing gaps
Many options above lack automated tests. Options marked with `[TODO](#testing-gaps)` need dedicated coverage in `tests/` before gaps can be closed.
