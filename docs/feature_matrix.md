# Feature Matrix

This table tracks the implementation status of rsync 3.4.x command-line options.
Behavioral differences from upstream rsync are tracked in [differences.md](differences.md) and outstanding parity gaps appear in [gaps.md](gaps.md).

Classic `rsync` protocol versions 31–32 are supported.

## Internal features

| Feature | Supported | Notes |
| --- | --- | --- |
| File list path-delta encoding with uid/gid tables | ✅ | Exercised via `filelist` tests |

| Option | Supported | Parity (Y/N) | Tests | Source | Notes |
| --- | --- | --- | --- | --- | --- |
| `--8-bit-output` | ✅ | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--acls` | ✅ | Y | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires `acl` feature |
| `--address` | ✅ | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--append` | ✅ | Y | [tests/resume.rs](../tests/resume.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--append-verify` | ✅ | Y | [tests/resume.rs](../tests/resume.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--archive` | ✅ | N | [tests/archive.rs](../tests/archive.rs)<br>[tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | composite flag; underlying gaps |
| `--atimes` | ✅ | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--backup` | ✅ | Y | [crates/engine/tests/backup.rs](../crates/engine/tests/backup.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | uses `~` suffix without `--backup-dir` |
| `--backup-dir` | ✅ | Y | [crates/engine/tests/backup.rs](../crates/engine/tests/backup.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | implies `--backup` |
| `--block-size` | ✅ | N | [tests/block_size.rs](../tests/block_size.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | controls delta block size |
| `--blocking-io` | ✅ | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--bwlimit` | ✅ | Y | [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | burst = 128×RATE bytes, min sleep = 100 ms |
| `--cc` | ✅ | Y | [tests/golden/cli_parity/checksum-choice.sh](../tests/golden/cli_parity/checksum-choice.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | alias for `--checksum-choice` |
| `--checksum` | ✅ | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | strong hashes: MD5 (default), SHA-1 |
| `--checksum-choice` | ✅ | Y | [tests/golden/cli_parity/checksum-choice.sh](../tests/golden/cli_parity/checksum-choice.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | choose the strong hash algorithm |
| `--checksum-seed` | ✅ | Y | [tests/checksum_seed.rs](../tests/checksum_seed.rs)<br>[tests/checksum_seed_cli.rs](../tests/checksum_seed_cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | set block/file checksum seed |
| `--chmod` | ✅ | Y | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/golden/cli_parity/chmod.sh](../tests/golden/cli_parity/chmod.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--chown` | ✅ | N | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires root or CAP_CHOWN |
| `--compare-dest` | ✅ | Y | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--compress` | ✅ | Y | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/compression_negotiation.sh](../tests/compression_negotiation.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | negotiates zstd when supported, else zlibx then zlib |
| `--compress-choice` | ✅ | Y | [tests/golden/cli_parity/compress-choice.sh](../tests/golden/cli_parity/compress-choice.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | supports zstd, zlibx, and zlib |
| `--compress-level` | ✅ | Y | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | applies to zlib or zstd |
| `--zc` | ✅ | Y | [tests/golden/cli_parity/compress-choice.sh](../tests/golden/cli_parity/compress-choice.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | alias for `--compress-choice` |
| `--zl` | ✅ | Y | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | alias for `--compress-level` |
| `--config` | ✅ | N | [tests/daemon_config.rs](../tests/daemon_config.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--contimeout` | ✅ | Y | [tests/timeout.rs](../tests/timeout.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--copy-as` | ✅ | N | [tests/copy_as.rs](../tests/copy_as.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires root or CAP_CHOWN |
| `--copy-dest` | ✅ | Y | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--copy-devices` | ✅ | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--copy-dirlinks` | ✅ | Y | [tests/golden/cli_parity/copy-dirlinks.sh](../tests/golden/cli_parity/copy-dirlinks.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--copy-links` | ✅ | N | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--copy-unsafe-links` | ✅ | N | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--crtimes` | ✅ | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--cvs-exclude` | ✅ | Y | [tests/cvs_exclude.rs](../tests/cvs_exclude.rs)<br>[tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--daemon` | ✅ | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--debug` | ✅ | Y | [crates/cli/tests/logging_flags.rs](../crates/cli/tests/logging_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--del` | ✅ | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | alias for `--delete-during` |
| `--delay-updates` | ✅ | Y | [tests/delay_updates.rs](../tests/delay_updates.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete` | ✅ | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete-after` | ✅ | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete-before` | ✅ | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete-delay` | ✅ | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete-during` | ✅ | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete-excluded` | ✅ | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete-missing-args` | ✅ | Y | [tests/delete_policy.rs](../tests/delete_policy.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--devices` | ✅ | Y | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--dirs` | ✅ | Y | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--dparam` | ❌ | N | — | — | not yet implemented |
| `--dry-run` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--early-input` | ✅ | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--exclude` | ✅ | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--exclude-from` | ✅ | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--executability` | ✅ | Y | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--existing` | ✅ | Y | [tests/filter_corpus.rs](../tests/filter_corpus.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--fake-super` | ✅ | N | [tests/fake_super.rs](../tests/fake_super.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires `xattr` feature |
| `--files-from` | ✅ | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--filter` | ✅ | Y | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--force` | ✅ | Y | [tests/delete_policy.rs](../tests/delete_policy.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--from0` | ✅ | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--fsync` | ✅ | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--fuzzy` | ✅ | N | [tests/fuzzy.rs](../tests/fuzzy.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--group` | ✅ | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires root or CAP_CHOWN |
| `--groupmap` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires root or CAP_CHOWN |
| `--hard-links` | ✅ | N | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--help` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--hosts-allow` | ✅ | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--hosts-deny` | ✅ | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--human-readable` | ✅ | Y | [tests/golden/cli_parity/human-readable.sh](../tests/golden/cli_parity/human-readable.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--iconv` | ❌ | N | — | — | not yet implemented |
| `--ignore-errors` | ✅ | N | [tests/delete_policy.rs](../tests/delete_policy.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--ignore-existing` | ✅ | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--ignore-missing-args` | ✅ | N | [tests/ignore_missing_args.rs](../tests/ignore_missing_args.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--ignore-times` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--include` | ✅ | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--include-from` | ✅ | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--info` | ✅ | N | [crates/logging/tests/info_flags.rs](../crates/logging/tests/info_flags.rs)<br>[crates/cli/tests/logging_flags.rs](../crates/cli/tests/logging_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--inplace` | ✅ | Y | [tests/golden/cli_parity/inplace.sh](../tests/golden/cli_parity/inplace.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--ipv4` | ✅ | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | select IPv4 transport or listener |
| `--ipv6` | ✅ | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | select IPv6 transport or listener |
| `--itemize-changes` | ✅ | Y | [tests/golden/cli_parity/itemize-changes.sh](../tests/golden/cli_parity/itemize-changes.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--keep-dirlinks` | ✅ | Y | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--link-dest` | ✅ | Y | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--links` | ✅ | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs)<br>[crates/engine/src/lib.rs](../crates/engine/src/lib.rs) | preserves relative/absolute targets; supports dangling links |
| `--list-only` | ✅ | Y | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--log-file` | ✅ | N | [tests/log_file.rs](../tests/log_file.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | limited format support |
| `--log-file-format` | ✅ | N | [tests/log_file.rs](../tests/log_file.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | limited format support |
| `--max-alloc` | ✅ | Y | [tests/perf_limits.rs](../tests/perf_limits.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--max-delete` | ✅ | Y | [tests/delete_policy.rs](../tests/delete_policy.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--max-size` | ✅ | N | [tests/perf_limits.rs](../tests/perf_limits.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--min-size` | ✅ | N | [tests/perf_limits.rs](../tests/perf_limits.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--mkpath` | ✅ | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--modify-window` | ✅ | N | [tests/modify_window.rs](../tests/modify_window.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | treat close mtimes as equal |
| `--motd` | ✅ | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--munge-links` | ❌ | N | — | — | not yet implemented |
| `--no-detach` | ❌ | N | — | — | not yet implemented |
| `--no-D` | ❌ | N | [gaps.md](gaps.md) | — | alias for `--no-devices --no-specials` |
| `--no-OPTION` | ❌ | N | — | — | not yet implemented |
| `--no-implied-dirs` | ✅ | Y | [tests/no_implied_dirs.rs](../tests/no_implied_dirs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | preserves existing symlinked directories |
| `--no-motd` | ✅ | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--numeric-ids` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--old-args` | ❌ | N | — | — | not yet implemented |
| `--old-d` | ❌ | N | [gaps.md](gaps.md) | — | alias for `--old-dirs` |
| `--old-dirs` | ❌ | N | — | — | not yet implemented |
| `--omit-dir-times` | ✅ | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--omit-link-times` | ✅ | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--one-file-system` | ❌ | N | — | — | not yet implemented |
| `--only-write-batch` | ❌ | N | — | — | not yet implemented |
| `--open-noatime` | ❌ | N | — | — | not yet implemented |
| `--out-format` | ❌ | N | — | — | not yet implemented |
| `--outbuf` | ❌ | N | — | — | not yet implemented |
| `--owner` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires root or CAP_CHOWN |
| `--partial` | ✅ | Y | [tests/cli.rs](../tests/cli.rs)<br>[crates/engine/tests/resume.rs](../crates/engine/tests/resume.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--partial-dir` | ✅ | Y | [tests/resume.rs](../tests/resume.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--password-file` | ✅ | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--perms` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--port` | ✅ | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | overrides default port |
| `--preallocate` | ✅ | Y | [tests/perf_limits.rs](../tests/perf_limits.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--progress` | ✅ | N | [tests/cli.rs#L309](../tests/cli.rs#L309) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--protocol` | ✅ | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--prune-empty-dirs` | ✅ | Y | [tests/filter_corpus.rs](../tests/filter_corpus.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--quiet` | ✅ | Y | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--read-batch` | ❌ | N | — | — | not yet implemented |
| `--recursive` | ✅ | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--relative` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--remote-option` | ✅ | N | [tests/interop/remote_option.rs](../tests/interop/remote_option.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--remove-source-files` | ✅ | Y | [tests/delete_policy.rs](../tests/delete_policy.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--rsh` | ✅ | Y | [tests/rsh.rs](../tests/rsh.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | supports quoting, env vars, and `RSYNC_RSH` |
| `--rsync-path` | ✅ | Y | [tests/rsh.rs](../tests/rsh.rs)<br>[tests/rsync_path.rs](../tests/rsync_path.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | accepts remote commands with env vars |
| `--safe-links` | ✅ | N | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--secluded-args` | ✅ | N | [tests/secluded_args.rs](../tests/secluded_args.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--secrets-file` | ✅ | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--server` | ✅ | N | [crates/protocol/tests/server.rs](../crates/protocol/tests/server.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | negotiates protocol version and codecs |
| `--size-only` | ✅ | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--skip-compress` | ✅ | Y | [tests/skip_compress.rs](../tests/skip_compress.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | comma-separated list of file suffixes to avoid compressing |
| `--sockopts` | ✅ | N | [tests/sockopts.rs](../tests/sockopts.rs)<br>[crates/transport/tests/sockopts.rs](../crates/transport/tests/sockopts.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | supports `SO_*` and `ip:ttl`/`ip:tos` |
| `--sparse` | ✅ | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | creates holes for long zero runs |
| `--specials` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--stats` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--stderr` | ❌ | N | — | — | not yet implemented |
| `--stop-after` | ❌ | N | — | — | not yet implemented |
| `--stop-at` | ❌ | N | — | — | not yet implemented |
| `--suffix` | ❌ | N | — | — | not yet implemented |
| `--super` | ✅ | N | [tests/cli.rs](../tests/cli.rs)<br>[crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | overrides `--fake-super` |
| `--temp-dir` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires same filesystem for atomic rename |
| `--timeout` | ✅ | N | [tests/timeout.rs](../tests/timeout.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | idle and I/O timeout |
| `--times` | ✅ | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--trust-sender` | ✅ | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | no-op; always trusts sender |
| `--update` | ✅ | N | [crates/engine/tests/update.rs](../crates/engine/tests/update.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--usermap` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires root or CAP_CHOWN |
| `--verbose` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--version` | ✅ | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--whole-file` | ✅ | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--write-batch` | ✅ | Y | [tests/write_batch.rs](../tests/write_batch.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--write-devices` | ✅ | Y | [tests/write_devices.rs](../tests/write_devices.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | writes to existing devices |
| `--xattrs` | ✅ | N | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires `xattr` feature; lacks parity |
