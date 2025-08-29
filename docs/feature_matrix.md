# Feature Matrix

This table tracks the implementation status of rsync 3.2.x command-line options.
See [differences.md](differences.md) for a summary of notable behavioral differences.

| Option | Supported | Parity | Tests | Notes | Enhanced? |
| --- | --- | --- | --- | --- | --- |
| `--8-bit-output` | ❌ | — | — |  | |
| `--acls` | ❌ | — | — |  | |
| `--address` | ❌ | — | — |  | |
| `--append` | ❌ | — | — |  | |
| `--append-verify` | ❌ | — | — |  | |
| `--archive` | ✅ | ❌ | [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh) |  | |
| `--atimes` | ❌ | — | — |  | |
| `--backup` | ❌ | — | — |  | |
| `--backup-dir` | ❌ | — | — |  | |
| `--block-size` | ❌ | — | — |  | |
| `--blocking-io` | ❌ | — | — |  | |
| `--bwlimit` | ❌ | — | — |  | |
| `--cc` | ❌ | — | [gaps.md](gaps.md) | alias for `--checksum-choice` | |
| `--checksum` | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs) | strong hashes: MD5 (default), SHA-1, BLAKE3 | |
| `--checksum-choice` | ❌ | — | — |  | |
| `--checksum-seed` | ❌ | — | — |  | |
| `--chmod` | ❌ | — | — |  | |
| `--chown` | ❌ | — | — |  | |
| `--compare-dest` | ❌ | — | — |  | |
| `--compress` | ✅ | ✅ | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh) |  | |
| `--compress-choice` | ❌ | — | — |  | |
| `--compress-level` | ✅ | ❌ | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) |  | |
| `--zc` | ❌ | — | [gaps.md](gaps.md) | alias for `--compress-choice` | |
| `--zl` | ❌ | — | [gaps.md](gaps.md) | alias for `--compress-level` | |
| `--contimeout` | ❌ | — | — |  | |
| `--copy-as` | ❌ | — | — |  | |
| `--copy-dest` | ❌ | — | — |  | |
| `--copy-devices` | ❌ | — | — |  | |
| `--copy-dirlinks` | ❌ | — | — |  | |
| `--copy-links` | ❌ | — | — |  | |
| `--copy-unsafe-links` | ❌ | — | — |  | |
| `--crtimes` | ❌ | — | — |  | |
| `--cvs-exclude` | ❌ | — | — |  | |
| `--daemon` | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | |
| `--debug` | ❌ | — | — |  | |
| `--del` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | alias for `--delete-during` | |
| `--delay-updates` | ❌ | — | — |  | |
| `--delete` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | |
| `--delete-after` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | |
| `--delete-before` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | |
| `--delete-delay` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--delete-during` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | |
| `--delete-excluded` | ❌ | — | — |  | |
| `--delete-missing-args` | ❌ | — | — |  | |
| `--devices` | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) |  | |
| `--dirs` | ❌ | — | — |  | |
| `--dry-run` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--early-input` | ❌ | — | — |  | |
| `--exclude` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--exclude-from` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--executability` | ❌ | — | — |  | |
| `--existing` | ❌ | — | — |  | |
| `--fake-super` | ❌ | — | — |  | |
| `--files-from` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--filter` | ✅ | ✅ | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | |
| `--force` | ❌ | — | — |  | |
| `--from0` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--fsync` | ❌ | — | — |  | |
| `--fuzzy` | ❌ | — | — |  | |
| `--group` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--groupmap` | ❌ | — | — |  | |
| `--hard-links` | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) |  | |
| `--help` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--human-readable` | ❌ | — | — |  | |
| `--iconv` | ❌ | — | — |  | |
| `--ignore-errors` | ❌ | — | — |  | |
| `--ignore-existing` | ❌ | — | — |  | |
| `--ignore-missing-args` | ❌ | — | — |  | |
| `--ignore-times` | ❌ | — | — |  | |
| `--include` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--include-from` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--info` | ❌ | — | — |  | |
| `--inplace` | ❌ | — | — |  | |
| `--ipv4` | ❌ | — | — |  | |
| `--ipv6` | ❌ | — | — |  | |
| `--itemize-changes` | ❌ | — | — |  | |
| `--keep-dirlinks` | ❌ | — | — |  | |
| `--link-dest` | ❌ | — | — |  | |
| `--links` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--list-only` | ❌ | — | — |  | |
| `--log-file` | ❌ | — | — |  | |
| `--log-file-format` | ❌ | — | — |  | |
| `--max-alloc` | ❌ | — | — |  | |
| `--max-delete` | ❌ | — | — |  | |
| `--max-size` | ❌ | — | — |  | |
| `--min-size` | ❌ | — | — |  | |
| `--mkpath` | ❌ | — | — |  | |
| `--modify-window` | ❌ | — | — |  | |
| `--munge-links` | ❌ | — | — |  | |
| `--no-D` | ❌ | — | [gaps.md](gaps.md) | alias for `--no-devices --no-specials` | |
| `--no-OPTION` | ❌ | — | — |  | |
| `--no-implied-dirs` | ❌ | — | — |  | |
| `--no-motd` | ❌ | — | — |  | |
| `--numeric-ids` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--old-args` | ❌ | — | — |  | |
| `--old-d` | ❌ | — | [gaps.md](gaps.md) | alias for `--old-dirs` | |
| `--old-dirs` | ❌ | — | — |  | |
| `--omit-dir-times` | ❌ | — | — |  | |
| `--omit-link-times` | ❌ | — | — |  | |
| `--one-file-system` | ❌ | — | — |  | |
| `--only-write-batch` | ❌ | — | — |  | |
| `--open-noatime` | ❌ | — | — |  | |
| `--out-format` | ❌ | — | — |  | |
| `--outbuf` | ❌ | — | — |  | |
| `--owner` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--partial` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--partial-dir` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--password-file` | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | |
| `--perms` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--port` | ❌ | — | — |  | |
| `--preallocate` | ❌ | — | — |  | |
| `--progress` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--protocol` | ❌ | — | — |  | |
| `--prune-empty-dirs` | ❌ | — | — |  | |
| `--quiet` | ✅ | ✅ | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | |
| `--read-batch` | ❌ | — | — |  | |
| `--recursive` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | |
| `--relative` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--remote-option` | ❌ | — | — |  | |
| `--remove-source-files` | ❌ | — | — |  | |
| `--rsh` | ❌ | — | [tests/rsh.rs](../tests/rsh.rs) |  | |
| `--rsync-path` | ❌ | — | [tests/rsync_path.rs](../tests/rsync_path.rs) |  | |
| `--safe-links` | ❌ | — | — |  | |
| `--secluded-args` | ❌ | — | — |  | |
| `--secrets-file` | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | |
| `--server` | ✅ | ❌ | [tests/server.rs](../tests/server.rs) | negotiates protocol version and codecs | |
| `--size-only` | ❌ | — | — |  | |
| `--skip-compress` | ❌ | — | — |  | |
| `--sockopts` | ❌ | — | — |  | |
| `--sparse` | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs) | creates holes for long zero runs | |
| `--specials` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--stats` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--stderr` | ❌ | — | — |  | |
| `--stop-after` | ❌ | — | — |  | |
| `--stop-at` | ❌ | — | — |  | |
| `--suffix` | ❌ | — | — |  | |
| `--super` | ❌ | — | — |  | |
| `--temp-dir` | ❌ | — | — |  | |
| `--timeout` | ❌ | — | — |  | |
| `--times` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--trust-sender` | ❌ | — | — |  | |
| `--update` | ❌ | — | — |  | |
| `--usermap` | ❌ | — | — |  | |
| `--verbose` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | |
| `--version` | ❌ | — | — |  | |
| `--whole-file` | ❌ | — | — |  | |
| `--write-batch` | ❌ | — | — |  | |
| `--write-devices` | ❌ | — | — |  | |
| `--xattrs` | ❌ | — | — |  | |
