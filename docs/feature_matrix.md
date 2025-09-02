# Feature Matrix

This table tracks the implementation status of rsync 3.4.x command-line options.
Behavioral differences from upstream rsync are tracked in [differences.md](differences.md) and detailed per-domain coverage appears in [gaps.md](gaps.md).

Classic `rsync` protocol versions 29–32 are supported.

## Internal features

| Feature | Supported | Notes |
| --- | --- | --- |
| File list path-delta encoding with uid/gid tables | ✅ | Exercised via `filelist` tests |

| Option | Supported | Parity (Y/N) | Message-parity (Y/N) | Parser-parity (Y/N) | Tests | Source | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `--8-bit-output` | ✅ | N | N | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--acls` | ✅ | Y | Y | Y | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires `acl` feature |
| `--address` | ✅ | Y | Y | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--append` | ✅ | Y | Y | Y | [tests/resume.rs](../tests/resume.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--append-verify` | ✅ | Y | Y | Y | [tests/resume.rs](../tests/resume.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--archive` | ✅ | N | N | N | [tests/archive.rs](../tests/archive.rs)<br>[tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | composite flag; underlying gaps |
| `--atimes` | ✅ | Y | Y | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--backup` | ✅ | Y | Y | Y | [crates/engine/tests/backup.rs](../crates/engine/tests/backup.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | uses `~` suffix without `--backup-dir` |
| `--backup-dir` | ✅ | Y | Y | Y | [crates/engine/tests/backup.rs](../crates/engine/tests/backup.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | implies `--backup` |
| `--block-size` | ✅ | N | N | N | [tests/block_size.rs](../tests/block_size.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | controls delta block size |
| `--blocking-io` | ✅ | N | N | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--bwlimit` | ✅ | Y | Y | Y | [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | burst = 128×RATE bytes, min sleep = 100 ms |
| `--cc` | ✅ | Y | Y | Y | [tests/golden/cli_parity/checksum-choice.sh](../tests/golden/cli_parity/checksum-choice.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | alias for `--checksum-choice` |
| `--checksum` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | strong hashes: MD5 (default), SHA-1 |
| `--checksum-choice` | ✅ | Y | Y | Y | [tests/golden/cli_parity/checksum-choice.sh](../tests/golden/cli_parity/checksum-choice.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | choose the strong hash algorithm |
| `--checksum-seed` | ✅ | Y | Y | Y | [tests/checksum_seed.rs](../tests/checksum_seed.rs)<br>[tests/checksum_seed_cli.rs](../tests/checksum_seed_cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | set block/file checksum seed |
| `--chmod` | ✅ | Y | Y | Y | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/golden/cli_parity/chmod.sh](../tests/golden/cli_parity/chmod.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--chown` | ✅ | N | N | N | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires root or CAP_CHOWN |
| `--compare-dest` | ✅ | Y | Y | Y | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--compress` | ✅ | Y | Y | Y | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/compression_negotiation.sh](../tests/compression_negotiation.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | negotiates zstd when supported, else zlibx then zlib |
| `--compress-choice` | ✅ | Y | Y | Y | [tests/golden/cli_parity/compress-choice.sh](../tests/golden/cli_parity/compress-choice.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | supports zstd, zlibx, and zlib |
| `--compress-level` | ✅ | Y | Y | Y | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | applies to zlib or zstd |
| `--zc` | ✅ | Y | Y | Y | [tests/golden/cli_parity/compress-choice.sh](../tests/golden/cli_parity/compress-choice.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | alias for `--compress-choice` |
| `--zl` | ✅ | Y | Y | Y | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | alias for `--compress-level` |
| `--config` | ✅ | N | N | N | [tests/daemon_config.rs](../tests/daemon_config.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--contimeout` | ✅ | Y | Y | Y | [tests/timeout.rs](../tests/timeout.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--copy-as` | ✅ | N | N | N | [tests/copy_as.rs](../tests/copy_as.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires root or CAP_CHOWN |
| `--copy-dest` | ✅ | Y | Y | Y | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--copy-devices` | ✅ | Y | Y | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--copy-dirlinks` | ✅ | Y | Y | Y | [tests/golden/cli_parity/copy-dirlinks.sh](../tests/golden/cli_parity/copy-dirlinks.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--copy-links` | ✅ | N | N | N | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--copy-unsafe-links` | ✅ | N | N | N | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--crtimes` | ✅ | Y | Y | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--cvs-exclude` | ✅ | Y | Y | Y | [tests/cvs_exclude.rs](../tests/cvs_exclude.rs)<br>[tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--daemon` | ✅ | Y | Y | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--debug` | ✅ | Y | Y | Y | [crates/cli/tests/logging_flags.rs](../crates/cli/tests/logging_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--del` | ✅ | Y | Y | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | alias for `--delete-during` |
| `--delay-updates` | ✅ | Y | Y | Y | [tests/delay_updates.rs](../tests/delay_updates.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete` | ✅ | Y | Y | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete-after` | ✅ | Y | Y | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete-before` | ✅ | Y | Y | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete-delay` | ✅ | Y | Y | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete-during` | ✅ | Y | Y | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete-excluded` | ✅ | Y | Y | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--delete-missing-args` | ✅ | Y | Y | Y | [tests/delete_policy.rs](../tests/delete_policy.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--devices` | ✅ | Y | Y | Y | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--dirs` | ✅ | Y | Y | Y | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--dparam` | ✅ | Y | Y | Y | [crates/cli/tests/cli_parity.rs](../crates/cli/tests/cli_parity.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | override global daemon config parameter |
| `--dry-run` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--early-input` | ✅ | N | N | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--exclude` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--exclude-from` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--executability` | ✅ | Y | Y | Y | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--existing` | ✅ | Y | Y | Y | [tests/filter_corpus.rs](../tests/filter_corpus.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--fake-super` | ✅ | N | N | N | [tests/fake_super.rs](../tests/fake_super.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires `xattr` feature |
| `--files-from` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--filter` | ✅ | Y | Y | Y | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--force` | ✅ | Y | Y | Y | [tests/delete_policy.rs](../tests/delete_policy.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--from0` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--fsync` | ✅ | N | N | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--fuzzy` | ✅ | N | N | N | [tests/fuzzy.rs](../tests/fuzzy.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--group` | ✅ | Y | Y | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires root or CAP_CHOWN |
| `--groupmap` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires root or CAP_CHOWN |
| `--hard-links` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs)<br>[crates/engine/tests/links.rs](../crates/engine/tests/links.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--help` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--hosts-allow` | ✅ | Y | Y | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--hosts-deny` | ✅ | Y | Y | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--human-readable` | ✅ | Y | Y | Y | [tests/golden/cli_parity/human-readable.sh](../tests/golden/cli_parity/human-readable.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--iconv` | ✅ | Y | Y | Y | [crates/cli/tests/iconv.rs](../crates/cli/tests/iconv.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | request charset conversion of filenames |
| `--ignore-errors` | ✅ | N | N | N | [tests/delete_policy.rs](../tests/delete_policy.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--ignore-existing` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--ignore-missing-args` | ✅ | N | N | N | [tests/ignore_missing_args.rs](../tests/ignore_missing_args.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--ignore-times` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--include` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--include-from` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--info` | ✅ | N | N | N | [crates/logging/tests/info_flags.rs](../crates/logging/tests/info_flags.rs)<br>[crates/cli/tests/logging_flags.rs](../crates/cli/tests/logging_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--inplace` | ✅ | Y | Y | Y | [tests/golden/cli_parity/inplace.sh](../tests/golden/cli_parity/inplace.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--ipv4` | ✅ | Y | Y | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | select IPv4 transport or listener |
| `--ipv6` | ✅ | Y | Y | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | select IPv6 transport or listener |
| `--itemize-changes` | ✅ | Y | Y | Y | [tests/golden/cli_parity/itemize-changes.sh](../tests/golden/cli_parity/itemize-changes.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--keep-dirlinks` | ✅ | Y | Y | Y | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--link-dest` | ✅ | Y | Y | Y | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--links` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs)<br>[crates/engine/src/lib.rs](../crates/engine/src/lib.rs) | preserves relative/absolute targets; supports dangling links |
| `--list-only` | ✅ | Y | Y | Y | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--log-file` | ✅ | N | N | N | [tests/log_file.rs](../tests/log_file.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | limited format support |
| `--log-file-format` | ✅ | N | N | N | [tests/log_file.rs](../tests/log_file.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | limited format support |
| `--max-alloc` | ✅ | Y | Y | Y | [tests/perf_limits.rs](../tests/perf_limits.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--max-delete` | ✅ | Y | Y | Y | [tests/delete_policy.rs](../tests/delete_policy.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--max-size` | ✅ | N | N | N | [tests/perf_limits.rs](../tests/perf_limits.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--min-size` | ✅ | N | N | N | [tests/perf_limits.rs](../tests/perf_limits.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--mkpath` | ✅ | N | N | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--modify-window` | ✅ | N | N | N | [tests/modify_window.rs](../tests/modify_window.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | treat close mtimes as equal |
| `--motd` | ✅ | Y | Y | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--munge-links` | ❌ | N | N | N | — | — | not yet implemented |
| `--no-detach` | ❌ | N | N | N | — | — | not yet implemented |
| `--no-D` | ❌ | N | N | N | [gaps.md](gaps.md) | — | alias for `--no-devices --no-specials` |
| `--no-OPTION` | ✅ | Y | Y | Y | [crates/cli/tests/cli_parity.rs](../crates/cli/tests/cli_parity.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | disable default option |
| `--no-implied-dirs` | ✅ | Y | Y | Y | [tests/no_implied_dirs.rs](../tests/no_implied_dirs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | preserves existing symlinked directories |
| `--no-motd` | ✅ | Y | Y | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--numeric-ids` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--old-args` | ❌ | N | N | N | — | — | not yet implemented |
| `--old-d` | ❌ | N | N | N | [gaps.md](gaps.md) | — | alias for `--old-dirs` |
| `--old-dirs` | ❌ | N | N | N | — | — | not yet implemented |
| `--omit-dir-times` | ✅ | Y | Y | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--omit-link-times` | ✅ | Y | Y | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--one-file-system` | ✅ | Y | Y | Y | [crates/walk/tests/walk.rs](../crates/walk/tests/walk.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--only-write-batch` | ❌ | N | N | N | — | — | not yet implemented |
| `--open-noatime` | ❌ | N | N | N | — | — | not yet implemented |
| `--out-format` | ❌ | N | N | N | — | — | not yet implemented |
| `--outbuf` | ❌ | N | N | N | — | — | not yet implemented |
| `--owner` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs)<br>[crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires root or CAP_CHOWN |
| `--partial` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs)<br>[crates/engine/tests/resume.rs](../crates/engine/tests/resume.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--partial-dir` | ✅ | Y | Y | Y | [tests/resume.rs](../tests/resume.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--password-file` | ✅ | Y | Y | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--perms` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs)<br>[crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--port` | ✅ | Y | Y | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | overrides default port |
| `--preallocate` | ✅ | Y | Y | Y | [tests/perf_limits.rs](../tests/perf_limits.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--progress` | ✅ | N | N | N | [tests/cli.rs#L309](../tests/cli.rs#L309) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--protocol` | ✅ | N | N | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--prune-empty-dirs` | ✅ | Y | Y | Y | [tests/filter_corpus.rs](../tests/filter_corpus.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--quiet` | ✅ | Y | Y | Y | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--read-batch` | ❌ | N | N | N | — | — | not yet implemented |
| `--recursive` | ✅ | Y | Y | Y | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--relative` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--remote-option` | ✅ | N | N | N | [tests/interop/remote_option.rs](../tests/interop/remote_option.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--remove-source-files` | ✅ | Y | Y | Y | [tests/delete_policy.rs](../tests/delete_policy.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--rsh` | ✅ | Y | Y | Y | [tests/rsh.rs](../tests/rsh.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | supports quoting, env vars, and `RSYNC_RSH` |
| `--rsync-path` | ✅ | Y | Y | Y | [tests/rsh.rs](../tests/rsh.rs)<br>[tests/rsync_path.rs](../tests/rsync_path.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | accepts remote commands with env vars |
| `--safe-links` | ✅ | N | N | N | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--secluded-args` | ✅ | N | N | N | [tests/secluded_args.rs](../tests/secluded_args.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--secrets-file` | ✅ | Y | Y | Y | [tests/daemon.rs](../tests/daemon.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--server` | ✅ | N | N | N | [crates/protocol/tests/server.rs](../crates/protocol/tests/server.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | negotiates protocol version and codecs |
| `--size-only` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--skip-compress` | ✅ | Y | Y | Y | [tests/skip_compress.rs](../tests/skip_compress.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | comma-separated list of file suffixes to avoid compressing |
| `--sockopts` | ✅ | N | N | N | [tests/sockopts.rs](../tests/sockopts.rs)<br>[crates/transport/tests/sockopts.rs](../crates/transport/tests/sockopts.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | supports `SO_*` and `ip:ttl`/`ip:tos` |
| `--sparse` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | creates holes for long zero runs |
| `--specials` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--stats` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--stderr` | ❌ | N | N | N | — | — | not yet implemented |
| `--stop-after` | ❌ | N | N | N | — | — | not yet implemented |
| `--stop-at` | ❌ | N | N | N | — | — | not yet implemented |
| `--suffix` | ❌ | N | N | N | — | — | not yet implemented |
| `--super` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs)<br>[crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | overrides `--fake-super` |
| `--temp-dir` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires same filesystem for atomic rename |
| `--timeout` | ✅ | N | N | N | [tests/timeout.rs](../tests/timeout.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | idle and I/O timeout |
| `--times` | ✅ | Y | Y | Y | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--trust-sender` | ✅ | N | N | N | [tests/cli_flags.rs](../tests/cli_flags.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | no-op; always trusts sender |
| `--update` | ✅ | N | N | N | [crates/engine/tests/update.rs](../crates/engine/tests/update.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--usermap` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires root or CAP_CHOWN |
| `--verbose` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--version` | ✅ | N | N | N | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--whole-file` | ✅ | Y | Y | Y | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--write-batch` | ✅ | Y | Y | Y | [tests/write_batch.rs](../tests/write_batch.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |  |
| `--write-devices` | ✅ | Y | Y | Y | [tests/write_devices.rs](../tests/write_devices.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | writes to existing devices |
| `--xattrs` | ✅ | N | N | N | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) | requires `xattr` feature |
