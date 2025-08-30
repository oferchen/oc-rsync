# Feature Matrix

This table tracks the implementation status of rsync 3.2.x command-line options.
See [differences.md](differences.md) for a summary of notable behavioral differences.

| Option | Short | Supported | Parity scope | Tests link | Notes | Version introduced |
| --- | --- | --- | --- | --- | --- | --- |
| `--8-bit-output` | `-8` | ❌ | — | — |  | ≤3.2 |
| `--acls` | `-A` | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) | requires `acl` feature | ≤3.2 |
| `--address` | — | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | ≤3.2 |
| `--append` | — | ✅ | ❌ | [tests/resume.rs](../tests/resume.rs) |  | ≤3.2 |
| `--append-verify` | — | ✅ | ❌ | [tests/resume.rs](../tests/resume.rs) |  | ≤3.2 |
| `--archive` | `-a` | ✅ | ❌ | [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh) |  | ≤3.2 |
| `--atimes` | `-U` | ✅ | ❌ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  | ≤3.2 |
| `--backup` | `-b` | ✅ | ✅ | [crates/engine/tests/backup.rs](../crates/engine/tests/backup.rs) | uses `~` suffix without `--backup-dir` | ≤3.2 |
| `--backup-dir` | — | ✅ | ✅ | [crates/engine/tests/backup.rs](../crates/engine/tests/backup.rs) | implies `--backup` | ≤3.2 |
| `--block-size` | `-B` | ✅ | ❌ | [tests/block_size.rs](../tests/block_size.rs) | controls delta block size | ≤3.2 |
| `--blocking-io` | — | ❌ | — | — |  | ≤3.2 |
| `--bwlimit` | — | ✅ | ❌ | [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs) |  | ≤3.2 |
| `--cc` | — | ✅ | ✅ | [tests/golden/cli_parity/checksum-choice.sh](../tests/golden/cli_parity/checksum-choice.sh) | alias for `--checksum-choice` | ≤3.2 |
| `--checksum` | `-c` | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs) | strong hashes: MD5 (default), SHA-1, BLAKE3 | ≤3.2 |
| `--checksum-choice` | — | ✅ | ✅ | [tests/golden/cli_parity/checksum-choice.sh](../tests/golden/cli_parity/checksum-choice.sh) | choose the strong hash algorithm | ≤3.2 |
| `--checksum-seed` | — | ❌ | — | — |  | ≤3.2 |
| `--chmod` | — | ❌ | — | — |  | ≤3.2 |
| `--chown` | — | ❌ | — | — |  | ≤3.2 |
| `--compare-dest` | — | ✅ | ✅ | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) |  | ≤3.2 |
| `--compress` | `-z` | ✅ | ✅ | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/compression_negotiation.sh](../tests/compression_negotiation.sh) | negotiates zstd when supported by both peers | ≤3.2 |
| `--compress-choice` | — | ✅ | ✅ | [tests/golden/cli_parity/compress-choice.sh](../tests/golden/cli_parity/compress-choice.sh) | supports zstd and zlib only | ≤3.2 |
| `--compress-level` | — | ✅ | ✅ | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) | applies to zlib or zstd | ≤3.2 |
| `--zc` | — | ✅ | ✅ | [tests/golden/cli_parity/compress-choice.sh](../tests/golden/cli_parity/compress-choice.sh) | alias for `--compress-choice` | ≤3.2 |
| `--zl` | — | ✅ | ✅ | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) | alias for `--compress-level` | ≤3.2 |
| `--contimeout` | — | ✅ | ❌ | [tests/timeout.rs](../tests/timeout.rs) |  | ≤3.2 |
| `--copy-as` | — | ❌ | — | — |  | ≤3.2 |
| `--copy-dest` | — | ✅ | ✅ | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) |  | ≤3.2 |
| `--copy-devices` | — | ❌ | — | — |  | ≤3.2 |
| `--copy-dirlinks` | `-k` | ✅ | ✅ | [tests/golden/cli_parity/copy-dirlinks.sh](../tests/golden/cli_parity/copy-dirlinks.sh) |  | ≤3.2 |
| `--copy-links` | `-L` | ✅ | ❌ | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) |  | ≤3.2 |
| `--copy-unsafe-links` | — | ✅ | ❌ | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) |  | ≤3.2 |
| `--crtimes` | `-N` | ✅ | ✅ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  | ≤3.2 |
| `--cvs-exclude` | `-C` | ❌ | — | — |  | ≤3.2 |
| `--daemon` | — | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | ≤3.2 |
| `--debug` | — | ❌ | — | — |  | ≤3.2 |
| `--del` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | alias for `--delete-during` | ≤3.2 |
| `--delay-updates` | — | ❌ | — | — |  | ≤3.2 |
| `--delete` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | ≤3.2 |
| `--delete-after` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | ≤3.2 |
| `--delete-before` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | ≤3.2 |
| `--delete-delay` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | ≤3.2 |
| `--delete-during` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | ≤3.2 |
| `--delete-excluded` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | ≤3.2 |
| `--delete-missing-args` | — | ❌ | — | — |  | ≤3.2 |
| `--devices` | — | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) |  | ≤3.2 |
| `--dirs` | `-d` | ❌ | — | — |  | ≤3.2 |
| `--dry-run` | `-n` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--early-input` | — | ❌ | — | — |  | ≤3.2 |
| `--exclude` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--exclude-from` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--executability` | `-E` | ❌ | — | — |  | ≤3.2 |
| `--existing` | — | ❌ | — | — |  | ≤3.2 |
| `--fake-super` | — | ❌ | — | — |  | ≤3.2 |
| `--files-from` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--filter` | `-f` | ✅ | ✅ | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | ≤3.2 |
| `--force` | — | ❌ | — | — |  | ≤3.2 |
| `--from0` | `-0` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--fsync` | — | ❌ | — | — |  | ≤3.2 |
| `--fuzzy` | `-y` | ❌ | — | — |  | ≤3.2 |
| `--group` | `-g` | ✅ | ✅ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  | ≤3.2 |
| `--groupmap` | — | ❌ | — | — |  | ≤3.2 |
| `--hard-links` | `-H` | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) |  | ≤3.2 |
| `--help` | `-h (*)` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--human-readable` | `-h` | ❌ | — | — |  | ≤3.2 |
| `--iconv` | — | ❌ | — | — |  | ≤3.2 |
| `--ignore-errors` | — | ❌ | — | — |  | ≤3.2 |
| `--ignore-existing` | — | ❌ | — | — |  | ≤3.2 |
| `--ignore-missing-args` | — | ❌ | — | — |  | ≤3.2 |
| `--ignore-times` | `-I` | ❌ | — | — |  | ≤3.2 |
| `--include` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--include-from` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--info` | — | ❌ | — | — |  | ≤3.2 |
| `--inplace` | — | ✅ | ✅ | [tests/golden/cli_parity/inplace.sh](../tests/golden/cli_parity/inplace.sh) |  | ≤3.2 |
| `--ipv4` | `-4` | ❌ | — | — |  | ≤3.2 |
| `--ipv6` | `-6` | ❌ | — | — |  | ≤3.2 |
| `--itemize-changes` | `-i` | ❌ | — | — |  | ≤3.2 |
| `--keep-dirlinks` | `-K` | ❌ | — | — |  | ≤3.2 |
| `--link-dest` | — | ✅ | ✅ | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) |  | ≤3.2 |
| `--links` | `-l` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--list-only` | — | ❌ | — | — |  | ≤3.2 |
| `--log-file` | — | ❌ | — | — |  | ≤3.2 |
| `--log-file-format` | — | ❌ | — | — |  | ≤3.2 |
| `--max-alloc` | — | ❌ | — | — |  | ≤3.2 |
| `--max-delete` | — | ❌ | — | — |  | ≤3.2 |
| `--max-size` | — | ❌ | — | — |  | ≤3.2 |
| `--min-size` | — | ❌ | — | — |  | ≤3.2 |
| `--mkpath` | — | ❌ | — | — |  | ≤3.2 |
| `--modern` | — | ✅ | ✅ | [tests/interop/modern.rs](../tests/interop/modern.rs) | rsync-rs only; enables zstd compression and BLAKE3 checksums | — |
| `--modify-window` | `-@` | ❌ | — | — |  | ≤3.2 |
| `--munge-links` | — | ❌ | — | — |  | ≤3.2 |
| `--no-D` | — | ❌ | — | [gaps.md](gaps.md) | alias for `--no-devices --no-specials` | ≤3.2 |
| `--no-OPTION` | — | ❌ | — | — |  | ≤3.2 |
| `--no-implied-dirs` | — | ❌ | — | — |  | ≤3.2 |
| `--no-motd` | — | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | ≤3.2 |
| `--numeric-ids` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--old-args` | — | ❌ | — | — |  | ≤3.2 |
| `--old-d` | — | ❌ | — | [gaps.md](gaps.md) | alias for `--old-dirs` | ≤3.2 |
| `--old-dirs` | — | ❌ | — | — |  | ≤3.2 |
| `--omit-dir-times` | `-O` | ❌ | — | — |  | ≤3.2 |
| `--omit-link-times` | `-J` | ❌ | — | — |  | ≤3.2 |
| `--one-file-system` | `-x` | ❌ | — | — |  | ≤3.2 |
| `--only-write-batch` | — | ❌ | — | — |  | ≤3.2 |
| `--open-noatime` | — | ❌ | — | — |  | ≤3.2 |
| `--out-format` | — | ❌ | — | — |  | ≤3.2 |
| `--outbuf` | — | ❌ | — | — |  | ≤3.2 |
| `--owner` | `-o` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--partial` | — | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs)<br>[crates/engine/tests/resume.rs](../crates/engine/tests/resume.rs) |  | ≤3.2 |
| `--partial-dir` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--password-file` | — | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | ≤3.2 |
| `--perms` | `-p` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--port` | — | ❌ | — | — |  | ≤3.2 |
| `--preallocate` | — | ❌ | — | — |  | ≤3.2 |
| `--progress` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--protocol` | — | ❌ | — | — |  | ≤3.2 |
| `--prune-empty-dirs` | `-m` | ❌ | — | — |  | ≤3.2 |
| `--quiet` | `-q` | ✅ | ✅ | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | ≤3.2 |
| `--read-batch` | — | ❌ | — | — |  | ≤3.2 |
| `--recursive` | `-r` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | ≤3.2 |
| `--relative` | `-R` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--remote-option` | `-M` | ❌ | — | — |  | ≤3.2 |
| `--remove-source-files` | — | ❌ | — | — |  | ≤3.2 |
| `--rsh` | `-e` | ⚠️ | ❌ | [tests/rsh.rs](../tests/rsh.rs) | negotiation incomplete; lacks full command parsing and environment handshake | ≤3.2 |
| `--rsync-path` | — | ⚠️ | ❌ | [tests/rsync_path.rs](../tests/rsync_path.rs) | requires `--rsh`; remote path negotiation incomplete | ≤3.2 |
| `--safe-links` | — | ✅ | ❌ | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) |  | ≤3.2 |
| `--secluded-args` | `-s` | ❌ | — | — |  | ≤3.2 |
| `--secrets-file` | — | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | ≤3.2 |
| `--server` | — | ✅ | ❌ | [tests/server.rs](../tests/server.rs) | negotiates protocol version and codecs | ≤3.2 |
| `--size-only` | — | ❌ | — | — |  | ≤3.2 |
| `--skip-compress` | — | ❌ | — | — |  | ≤3.2 |
| `--sockopts` | — | ❌ | — | — |  | ≤3.2 |
| `--sparse` | `-S` | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs) | creates holes for long zero runs | ≤3.2 |
| `--specials` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--stats` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--stderr` | — | ❌ | — | — |  | ≤3.2 |
| `--stop-after` | — | ❌ | — | — |  | ≤3.2 |
| `--stop-at` | — | ❌ | — | — |  | ≤3.2 |
| `--suffix` | — | ❌ | — | — |  | ≤3.2 |
| `--super` | — | ❌ | — | — |  | ≤3.2 |
| `--temp-dir` | `-T` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) | requires same filesystem for atomic rename | ≤3.2 |
| `--timeout` | — | ✅ | ❌ | [tests/timeout.rs](../tests/timeout.rs) |  | ≤3.2 |
| `--times` | `-t` | ✅ | ✅ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  | ≤3.2 |
| `--trust-sender` | — | ❌ | — | — |  | ≤3.2 |
| `--update` | `-u` | ❌ | — | — |  | ≤3.2 |
| `--usermap` | — | ❌ | — | — |  | ≤3.2 |
| `--verbose` | `-v` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--version` | `-V` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | ≤3.2 |
| `--whole-file` | `-W` | ❌ | — | — |  | ≤3.2 |
| `--write-batch` | — | ❌ | — | — |  | ≤3.2 |
| `--write-devices` | — | ❌ | — | — |  | ≤3.2 |
| `--xattrs` | `-X` | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) | requires `xattr` feature | ≤3.2 |
