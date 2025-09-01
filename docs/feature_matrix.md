# Feature Matrix

This table tracks the implementation status of rsync 3.4.x command-line options.
See [differences.md](differences.md) for a summary of notable behavioral differences and [gaps.md](gaps.md) for
outstanding parity gaps.

Classic `rsync` protocol versions 31–32 are supported, while modern mode
negotiates version 73.

## Internal features

| Feature | Supported | Notes |
| --- | --- | --- |
| File list path-delta encoding with uid/gid tables | ✅ | Exercised via `filelist` tests |

| Option | Short | Supported | Parity scope | Tests link | Notes | Version introduced |
| --- | --- | --- | --- | --- | --- | --- |
| `--8-bit-output` | `-8` | ✅ | ❌ | [tests/cli_flags.rs](../tests/cli_flags.rs) |  | ≤3.2 |
| `--acls` | `-A` | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) | requires `acl` feature | ≤3.2 |
| `--address` | — | ✅ | ✅ | [tests/daemon.rs](../tests/daemon.rs) |  | ≤3.2 |
| `--append` | — | ✅ | ❌ | [tests/resume.rs](../tests/resume.rs) |  | ≤3.2 |
| `--append-verify` | — | ✅ | ❌ | [tests/resume.rs](../tests/resume.rs) |  | ≤3.2 |
| `--archive` | `-a` | ✅ | ❌ | [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh) |  | ≤3.2 |
| `--atimes` | `-U` | ✅ | ❌ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  | ≤3.2 |
| `--backup` | `-b` | ✅ | ✅ | [crates/engine/tests/backup.rs](../crates/engine/tests/backup.rs) | uses `~` suffix without `--backup-dir` | ≤3.2 |
| `--backup-dir` | — | ✅ | ✅ | [crates/engine/tests/backup.rs](../crates/engine/tests/backup.rs) | implies `--backup` | ≤3.2 |
| `--block-size` | `-B` | ✅ | ❌ | [tests/block_size.rs](../tests/block_size.rs) | controls delta block size | ≤3.2 |
| `--blocking-io` | — | ✅ | ❌ | [tests/cli_flags.rs](../tests/cli_flags.rs) |  | ≤3.2 |
| `--bwlimit` | — | ✅ | ❌ | [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs) |  | ≤3.2 |
| `--cc` | — | ✅ | ✅ | [tests/golden/cli_parity/checksum-choice.sh](../tests/golden/cli_parity/checksum-choice.sh) | alias for `--checksum-choice` | ≤3.2 |
| `--checksum` | `-c` | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs) | strong hashes: MD5 (default), SHA-1, BLAKE3 | ≤3.2 |
| `--checksum-choice` | — | ✅ | ✅ | [tests/golden/cli_parity/checksum-choice.sh](../tests/golden/cli_parity/checksum-choice.sh) | choose the strong hash algorithm | ≤3.2 |
| `--checksum-seed` | — | ✅ | ✅ | [tests/checksum_seed.rs](../tests/checksum_seed.rs)<br>[tests/checksum_seed_cli.rs](../tests/checksum_seed_cli.rs) | set block/file checksum seed | ≤3.2 |
| `--chmod` | — | ✅ | ✅ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/golden/cli_parity/chmod.sh](../tests/golden/cli_parity/chmod.sh) |  | ≤3.2 |
| `--chown` | — | ✅ | ❌ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | requires root or CAP_CHOWN | ≤3.2 |
| `--compare-dest` | — | ✅ | ✅ | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) |  | ≤3.2 |
| `--compress` | `-z` | ✅ | ✅ | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/compression_negotiation.sh](../tests/compression_negotiation.sh) | negotiates zstd when supported by both peers | ≤3.2 |
| `--compress-choice` | — | ✅ | ✅ | [tests/golden/cli_parity/compress-choice.sh](../tests/golden/cli_parity/compress-choice.sh) | supports zstd and zlib only | ≤3.2 |
| `--compress-level` | — | ✅ | ✅ | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) | applies to zlib or zstd | ≤3.2 |
| `--zc` | — | ✅ | ✅ | [tests/golden/cli_parity/compress-choice.sh](../tests/golden/cli_parity/compress-choice.sh) | alias for `--compress-choice` | ≤3.2 |
| `--zl` | — | ✅ | ✅ | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) | alias for `--compress-level` | ≤3.2 |
| `--config` | — | ✅ | ❌ | [tests/daemon_config.rs](../tests/daemon_config.rs) |  | ≤3.2 |
| `--contimeout` | — | ✅ | ❌ | [tests/timeout.rs](../tests/timeout.rs) |  | ≤3.2 |
| `--copy-as` | — | ✅ | ❌ | [tests/copy_as.rs](../tests/copy_as.rs) | requires root or CAP_CHOWN | ≤3.2 |
| `--copy-dest` | — | ✅ | ✅ | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) |  | ≤3.2 |
| `--copy-devices` | — | ✅ | ✅ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  | ≤3.2 |
| `--copy-dirlinks` | `-k` | ✅ | ✅ | [tests/golden/cli_parity/copy-dirlinks.sh](../tests/golden/cli_parity/copy-dirlinks.sh) |  | ≤3.2 |
| `--copy-links` | `-L` | ✅ | ❌ | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) |  | ≤3.2 |
| `--copy-unsafe-links` | — | ✅ | ❌ | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) |  | ≤3.2 |
| `--crtimes` | `-N` | ✅ | ✅ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  | ≤3.2 |
| `--cvs-exclude` | `-C` | ✅ | ✅ | [tests/cvs_exclude.rs](../tests/cvs_exclude.rs)<br>[tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--daemon` | — | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | ≤3.2 |
| `--debug` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--del` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | alias for `--delete-during` | ≤3.2 |
| `--delay-updates` | — | ✅ | ✅ | [tests/delay_updates.rs](../tests/delay_updates.rs) |  | ≤3.2 |
| `--delete` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | ≤3.2 |
| `--delete-after` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | ≤3.2 |
| `--delete-before` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | ≤3.2 |
| `--delete-delay` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | ≤3.2 |
| `--delete-during` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | ≤3.2 |
| `--delete-excluded` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | ≤3.2 |
| `--delete-missing-args` | — | ✅ | ✅ | [tests/delete_policy.rs](../tests/delete_policy.rs) |  | ≤3.2 |
| `--devices` | — | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) |  | ≤3.2 |
| `--dirs` | `-d` | ✅ | ✅ | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | ≤3.2 |
| `--dparam` | `-M` | ❌ | — | — | not yet implemented | ≤3.2 |
| `--dry-run` | `-n` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--early-input` | — | ✅ | ❌ | [tests/cli_flags.rs](../tests/cli_flags.rs) |  | ≤3.2 |
| `--exclude` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--exclude-from` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--executability` | `-E` | ✅ | ✅ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) |  | ≤3.2 |
| `--existing` | — | ✅ | ✅ | [tests/filter_corpus.rs](../tests/filter_corpus.rs) |  | ≤3.2 |
| `--fake-super` | — | ✅ | ❌ | [tests/fake_super.rs](../tests/fake_super.rs) | requires `xattr` feature | ≤3.2 |
| `--files-from` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--filter` | `-f` | ✅ | ✅ | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | ≤3.2 |
| `--force` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--from0` | `-0` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--fsync` | — | ✅ | ❌ | [tests/cli_flags.rs](../tests/cli_flags.rs) |  | ≤3.2 |
| `--fuzzy` | `-y` | ✅ | ❌ | [tests/fuzzy.rs](../tests/fuzzy.rs) |  | ≤3.2 |
| `--group` | `-g` | ✅ | ✅ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) | requires root or CAP_CHOWN | ≤3.2 |
| `--groupmap` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) | numeric gid mapping only; requires root or CAP_CHOWN | ≤3.2 |
| `--hard-links` | `-H` | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) |  | ≤3.2 |
| `--help` | `-h (*)` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--human-readable` |  | ✅ | ✅ | [tests/golden/cli_parity/human-readable.sh](../tests/golden/cli_parity/human-readable.sh) |  | ≤3.2 |
| `--iconv` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--ignore-errors` | — | ✅ | ❌ | [tests/delete_policy.rs](../tests/delete_policy.rs) |  | ≤3.2 |
| `--ignore-existing` | — | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--ignore-missing-args` | — | ✅ | ❌ | [tests/ignore_missing_args.rs](../tests/ignore_missing_args.rs) |  | ≤3.2 |
| `--ignore-times` | `-I` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--include` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--include-from` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--info` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--inplace` | — | ✅ | ✅ | [tests/golden/cli_parity/inplace.sh](../tests/golden/cli_parity/inplace.sh) |  | ≤3.2 |
| `--ipv4` | `-4` | ✅ | ✅ | [tests/daemon.rs](../tests/daemon.rs) | select IPv4 transport or listener | ≤3.2 |
| `--ipv6` | `-6` | ✅ | ✅ | [tests/daemon.rs](../tests/daemon.rs) | select IPv6 transport or listener | ≤3.2 |
| `--itemize-changes` | `-i` | ✅ | ✅ | [tests/golden/cli_parity/itemize-changes.sh](../tests/golden/cli_parity/itemize-changes.sh) |  | 3.2 |
| `--keep-dirlinks` | `-K` | ✅ | ✅ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) |  | ≤3.2 |
| `--link-dest` | — | ✅ | ✅ | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) |  | ≤3.2 |
| `--links` | `-l` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--list-only` | — | ✅ | ✅ | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | ≤3.2 |
| `--log-file` | — | ✅ | ❌ | [tests/log_file.rs](../tests/log_file.rs) | limited format support | ≤3.2 |
| `--log-file-format` | — | ✅ | ❌ | [tests/log_file.rs](../tests/log_file.rs) | limited format support | ≤3.2 |
| `--max-alloc` | — | ✅ | ✅ | [tests/perf_limits.rs](../tests/perf_limits.rs) |  | ≤3.2 |
| `--max-delete` | — | ✅ | ✅ | [tests/delete_policy.rs](../tests/delete_policy.rs) |  | ≤3.2 |
| `--max-size` | — | ✅ | ❌ | [tests/perf_limits.rs](../tests/perf_limits.rs) |  | ≤3.2 |
| `--min-size` | — | ✅ | ❌ | [tests/perf_limits.rs](../tests/perf_limits.rs) |  | ≤3.2 |
| `--mkpath` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--modern` | — | ✅ | ✅ | [tests/interop/modern.rs](../tests/interop/modern.rs) | oc-rsync only; enables zstd compression and BLAKE3 checksums (requires `blake3` feature) | — |
| `--modern-compress` | — | ✅ | ✅ | [tests/golden/cli_parity/modern_flags.sh](../tests/golden/cli_parity/modern_flags.sh) | oc-rsync only; choose `auto`, `zstd`, or `lz4` compression | — |
| `--modern-hash` | — | ✅ | ✅ | [tests/golden/cli_parity/modern_flags.sh](../tests/golden/cli_parity/modern_flags.sh) | oc-rsync only; select BLAKE3 hash (requires `blake3` feature) | — |
| `--modern-cdc` | — | ✅ | ✅ | [tests/golden/cli_parity/modern_flags.sh](../tests/golden/cli_parity/modern_flags.sh) | oc-rsync only; enable `fastcdc` chunking | — |
| `--modern-cdc-min` | — | ✅ | ✅ | [tests/cdc.rs](../tests/cdc.rs) | oc-rsync only; set FastCDC minimum chunk size | — |
| `--modern-cdc-max` | — | ✅ | ✅ | [tests/cdc.rs](../tests/cdc.rs) | oc-rsync only; set FastCDC maximum chunk size | — |
| `--modify-window` | `-@` | ✅ | ❌ | [tests/modify_window.rs](../tests/modify_window.rs) | treat close mtimes as equal | ≤3.2 |
| `--munge-links` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--no-detach` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--no-D` | — | ❌ | — | [gaps.md](gaps.md) | alias for `--no-devices --no-specials` | ≤3.2 |
| `--no-OPTION` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--no-implied-dirs` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--no-motd` | — | ✅ | ✅ | [tests/daemon.rs](../tests/daemon.rs) |  | ≤3.2 |
| `--numeric-ids` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--old-args` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--old-d` | — | ❌ | — | [gaps.md](gaps.md) | alias for `--old-dirs` | ≤3.2 |
| `--old-dirs` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--omit-dir-times` | `-O` | ✅ | ✅ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  | ≤3.2 |
| `--omit-link-times` | `-J` | ✅ | ✅ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  | ≤3.2 |
| `--one-file-system` | `-x` | ❌ | — | — | not yet implemented | ≤3.2 |
| `--only-write-batch` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--open-noatime` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--out-format` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--outbuf` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--owner` | `-o` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) | requires root or CAP_CHOWN | ≤3.2 |
| `--partial` | — | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs)<br>[crates/engine/tests/resume.rs](../crates/engine/tests/resume.rs) |  | ≤3.2 |
| `--partial-dir` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--password-file` | — | ✅ | ✅ | [tests/daemon.rs](../tests/daemon.rs) |  | ≤3.2 |
| `--perms` | `-p` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--port` | — | ✅ | ✅ | [tests/daemon.rs](../tests/daemon.rs) | overrides default port | ≤3.2 |
| `--preallocate` | — | ✅ | ✅ | [tests/perf_limits.rs](../tests/perf_limits.rs) |  | ≤3.2 |
| `--progress` | — | ✅ | ❌ | [tests/cli.rs#L309](../tests/cli.rs#L309) |  | ≤3.2 |
| `--protocol` | — | ✅ | ❌ | [tests/cli_flags.rs](../tests/cli_flags.rs) |  | ≤3.2 |
| `--prune-empty-dirs` | `-m` | ✅ | ✅ | [tests/filter_corpus.rs](../tests/filter_corpus.rs) |  | ≤3.2 |
| `--quiet` | `-q` | ✅ | ✅ | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | ≤3.2 |
| `--read-batch` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--recursive` | `-r` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | ≤3.2 |
| `--relative` | `-R` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--remote-option` | `-M` | ✅ | ❌ | [tests/interop/remote_option.rs](../tests/interop/remote_option.rs) |  | ≤3.2 |
| `--remove-source-files` | — | ✅ | ✅ | [tests/delete_policy.rs](../tests/delete_policy.rs) |  | ≤3.2 |
| `--rsh` | `-e` | ✅ | ✅ | [tests/rsh.rs](../tests/rsh.rs) | supports quoting, env vars, and `RSYNC_RSH` | ≤3.2 |
| `--rsync-path` | — | ✅ | ✅ | [tests/rsh.rs](../tests/rsh.rs)<br>[tests/rsync_path.rs](../tests/rsync_path.rs) | accepts remote commands with env vars | ≤3.2 |
| `--safe-links` | — | ✅ | ❌ | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) |  | ≤3.2 |
| `--secluded-args` | `-s` | ✅ | ❌ | [tests/secluded_args.rs](../tests/secluded_args.rs) |  | ≤3.2 |
| `--secrets-file` | — | ✅ | ✅ | [tests/daemon.rs](../tests/daemon.rs) |  | ≤3.2 |
| `--server` | — | ✅ | ❌ | [tests/server.rs](../tests/server.rs) | negotiates protocol version and codecs | ≤3.2 |
| `--size-only` | — | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--skip-compress` | — | ✅ | ✅ | [tests/skip_compress.rs](../tests/skip_compress.rs) | comma-separated list of file suffixes to avoid compressing | ≤3.2 |
| `--sockopts` | — | ✅ | ❌ | [tests/sockopts.rs](../tests/sockopts.rs)<br>[crates/transport/tests/sockopts.rs](../crates/transport/tests/sockopts.rs) | supports `SO_*` and `ip:ttl`/`ip:tos` | ≤3.2 |
| `--sparse` | `-S` | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs) | creates holes for long zero runs | ≤3.2 |
| `--specials` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--stats` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--stderr` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--stop-after` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--stop-at` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--suffix` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--super` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--temp-dir` | `-T` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) | requires same filesystem for atomic rename | ≤3.2 |
| `--timeout` | — | ✅ | ❌ | [tests/timeout.rs](../tests/timeout.rs) |  | ≤3.2 |
| `--times` | `-t` | ✅ | ✅ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  | ≤3.2 |
| `--trust-sender` | — | ❌ | — | — | not yet implemented | ≤3.2 |
| `--update` | `-u` | ✅ | ❌ | [crates/engine/tests/update.rs](../crates/engine/tests/update.rs) |  | ≤3.2 |
| `--usermap` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) | numeric uid mapping only; requires root or CAP_CHOWN | ≤3.2 |
| `--verbose` | `-v` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--version` | `-V` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--whole-file` | `-W` | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--write-batch` | — | ✅ | ✅ | [tests/write_batch.rs](../tests/write_batch.rs) |  | ≤3.2 |
| `--write-devices` | — | ✅ | ✅ | [tests/write_devices.rs](../tests/write_devices.rs) | writes to existing devices | ≤3.2 |
| `--xattrs` | `-X` | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) | requires `xattr` feature | ≤3.2 |
