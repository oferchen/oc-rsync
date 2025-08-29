# Feature Matrix

This table tracks the implementation status of rsync 3.4.1 command-line options.
See [differences.md](differences.md) for a summary of notable behavioral differences.

| Option | Short | Supported | Parity scope | Tests link | Notes | Version introduced |
| --- | --- | --- | --- | --- | --- | --- |
| `--8-bit-output` | `-8` | ❌ | — | — |  | — |
| `--acls` | `-A` | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) | requires `acl` feature | — |
| `--address` | — | ❌ | — | — |  | — |
| `--append` | — | ❌ | — | — |  | — |
| `--append-verify` | — | ❌ | — | — |  | — |
| `--archive` | `-a` | ✅ | ❌ | [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh) |  | — |
| `--atimes` | `-U` | ✅ | ❌ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  | — |
| `--backup` | `-b` | ❌ | — | — |  | — |
| `--backup-dir` | — | ❌ | — | — |  | — |
| `--block-size` | `-B` | ❌ | — | — |  | — |
| `--blocking-io` | — | ❌ | — | — |  | — |
| `--bwlimit` | — | ✅ | ❌ | [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs) |  | — |
| `--cc` | — | ❌ | — | [gaps.md](gaps.md) | alias for `--checksum-choice` | — |
| `--checksum` | `-c` | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs) | strong hashes: MD5 (default), SHA-1, BLAKE3 | — |
| `--checksum-choice` | — | ❌ | — | — |  | — |
| `--checksum-seed` | — | ❌ | — | — |  | — |
| `--chmod` | — | ❌ | — | — |  | — |
| `--chown` | — | ❌ | — | — |  | — |
| `--compare-dest` | — | ✅ | ✅ | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) |  | — |
| `--compress` | `-z` | ✅ | ✅ | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/compression_negotiation.sh](../tests/compression_negotiation.sh) | negotiates zstd when supported by both peers | — |
| `--compress-choice` | — | ✅ | ✅ | [tests/golden/cli_parity/compress-choice.sh](../tests/golden/cli_parity/compress-choice.sh) | supports zstd and zlib only | — |
| `--compress-level` | — | ✅ | ✅ | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) | applies to zlib or zstd | — |
| `--zc` | — | ✅ | ✅ | [tests/golden/cli_parity/compress-choice.sh](../tests/golden/cli_parity/compress-choice.sh) | alias for `--compress-choice` | — |
| `--zl` | — | ✅ | ✅ | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) | alias for `--compress-level` | — |
| `--contimeout` | — | ❌ | — | — |  | — |
| `--copy-as` | — | ❌ | — | — |  | — |
| `--copy-dest` | — | ✅ | ✅ | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) |  | — |
| `--copy-devices` | — | ❌ | — | — |  | — |
| `--copy-dirlinks` | `-k` | ❌ | — | — |  | — |
| `--copy-links` | `-L` | ❌ | — | — |  | — |
| `--copy-unsafe-links` | — | ❌ | — | — |  | — |
| `--crtimes` | `-N` | ✅ | ❌ | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  | — |
| `--cvs-exclude` | `-C` | ❌ | — | — |  | — |
| `--daemon` | — | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | — |
| `--debug` | — | ❌ | — | — |  | — |
| `--del` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | alias for `--delete-during` | — |
| `--delay-updates` | — | ❌ | — | — |  | — |
| `--delete` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | — |
| `--delete-after` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | — |
| `--delete-before` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | — |
| `--delete-delay` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | — |
| `--delete-during` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | — |
| `--delete-excluded` | — | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  | — |
| `--delete-missing-args` | — | ❌ | — | — |  | — |
| `--devices` | — | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) |  | — |
| `--dirs` | `-d` | ❌ | — | — |  | — |
| `--dry-run` | `-n` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--early-input` | — | ❌ | — | — |  | — |
| `--exclude` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--exclude-from` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--executability` | `-E` | ❌ | — | — |  | — |
| `--existing` | — | ❌ | — | — |  | — |
| `--fake-super` | — | ❌ | — | — |  | — |
| `--files-from` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--filter` | `-f` | ✅ | ✅ | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | — |
| `--force` | — | ❌ | — | — |  | — |
| `--from0` | `-0` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--fsync` | — | ❌ | — | — |  | — |
| `--fuzzy` | `-y` | ❌ | — | — |  | — |
| `--group` | `-g` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--groupmap` | — | ❌ | — | — |  | — |
| `--hard-links` | `-H` | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) |  | — |
| `--help` | `-h (*)` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--human-readable` | `-h` | ❌ | — | — |  | — |
| `--iconv` | — | ❌ | — | — |  | — |
| `--ignore-errors` | — | ❌ | — | — |  | — |
| `--ignore-existing` | — | ❌ | — | — |  | — |
| `--ignore-missing-args` | — | ❌ | — | — |  | — |
| `--ignore-times` | `-I` | ❌ | — | — |  | — |
| `--include` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--include-from` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--info` | — | ❌ | — | — |  | — |
| `--inplace` | — | ✅ | ✅ | [tests/golden/cli_parity/inplace.sh](../tests/golden/cli_parity/inplace.sh) |  | — |
| `--ipv4` | `-4` | ❌ | — | — |  | — |
| `--ipv6` | `-6` | ❌ | — | — |  | — |
| `--itemize-changes` | `-i` | ❌ | — | — |  | — |
| `--keep-dirlinks` | `-K` | ❌ | — | — |  | — |
| `--link-dest` | — | ✅ | ✅ | [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs) |  | — |
| `--links` | `-l` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--list-only` | — | ❌ | — | — |  | — |
| `--log-file` | — | ❌ | — | — |  | — |
| `--log-file-format` | — | ❌ | — | — |  | — |
| `--max-alloc` | — | ❌ | — | — |  | — |
| `--max-delete` | — | ❌ | — | — |  | — |
| `--max-size` | — | ❌ | — | — |  | — |
| `--min-size` | — | ❌ | — | — |  | — |
| `--mkpath` | — | ❌ | — | — |  | — |
| `--modify-window` | `-@` | ❌ | — | — |  | — |
| `--munge-links` | — | ❌ | — | — |  | — |
| `--no-D` | — | ❌ | — | [gaps.md](gaps.md) | alias for `--no-devices --no-specials` | — |
| `--no-OPTION` | — | ❌ | — | — |  | — |
| `--no-implied-dirs` | — | ❌ | — | — |  | — |
| `--no-motd` | — | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | — |
| `--numeric-ids` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--old-args` | — | ❌ | — | — |  | — |
| `--old-d` | — | ❌ | — | [gaps.md](gaps.md) | alias for `--old-dirs` | — |
| `--old-dirs` | — | ❌ | — | — |  | — |
| `--omit-dir-times` | `-O` | ❌ | — | — |  | — |
| `--omit-link-times` | `-J` | ❌ | — | — |  | — |
| `--one-file-system` | `-x` | ❌ | — | — |  | — |
| `--only-write-batch` | — | ❌ | — | — |  | — |
| `--open-noatime` | — | ❌ | — | — |  | — |
| `--out-format` | — | ❌ | — | — |  | — |
| `--outbuf` | — | ❌ | — | — |  | — |
| `--owner` | `-o` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--partial` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--partial-dir` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--password-file` | — | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | — |
| `--perms` | `-p` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--port` | — | ❌ | — | — |  | — |
| `--preallocate` | — | ❌ | — | — |  | — |
| `--progress` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--protocol` | — | ❌ | — | — |  | — |
| `--prune-empty-dirs` | `-m` | ❌ | — | — |  | — |
| `--quiet` | `-q` | ✅ | ✅ | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | — |
| `--read-batch` | — | ❌ | — | — |  | — |
| `--recursive` | `-r` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  | — |
| `--relative` | `-R` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--remote-option` | `-M` | ❌ | — | — |  | — |
| `--remove-source-files` | — | ❌ | — | — |  | — |
| `--rsh` | `-e` | ❌ | — | [tests/rsh.rs](../tests/rsh.rs) |  | — |
| `--rsync-path` | — | ❌ | — | [tests/rsync_path.rs](../tests/rsync_path.rs) |  | — |
| `--safe-links` | — | ❌ | — | — |  | — |
| `--secluded-args` | `-s` | ❌ | — | — |  | — |
| `--secrets-file` | — | ✅ | ❌ | [tests/daemon.rs](../tests/daemon.rs) |  | — |
| `--server` | — | ✅ | ❌ | [tests/server.rs](../tests/server.rs) | negotiates protocol version and codecs | — |
| `--size-only` | — | ❌ | — | — |  | — |
| `--skip-compress` | — | ❌ | — | — |  | — |
| `--sockopts` | — | ❌ | — | — |  | — |
| `--sparse` | `-S` | ✅ | ✅ | [tests/cli.rs](../tests/cli.rs) | creates holes for long zero runs | — |
| `--specials` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--stats` | — | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--stderr` | — | ❌ | — | — |  | — |
| `--stop-after` | — | ❌ | — | — |  | — |
| `--stop-at` | — | ❌ | — | — |  | — |
| `--suffix` | — | ❌ | — | — |  | — |
| `--super` | — | ❌ | — | — |  | — |
| `--temp-dir` | `-T` | ❌ | — | — |  | — |
| `--timeout` | — | ❌ | — | — |  | — |
| `--times` | `-t` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--trust-sender` | — | ❌ | — | — |  | — |
| `--update` | `-u` | ❌ | — | — |  | — |
| `--usermap` | — | ❌ | — | — |  | — |
| `--verbose` | `-v` | ✅ | ❌ | [tests/cli.rs](../tests/cli.rs) |  | — |
| `--version` | `-V` | ❌ | — | — |  | — |
| `--whole-file` | `-W` | ❌ | — | — |  | — |
| `--write-batch` | — | ❌ | — | — |  | — |
| `--write-devices` | — | ❌ | — | — |  | — |
| `--xattrs` | `-X` | ✅ | ❌ | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) | requires `xattr` feature | — |
