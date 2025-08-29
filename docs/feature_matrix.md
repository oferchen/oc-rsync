# Feature Matrix

This table tracks the implementation status of rsync 3.2.x command-line options.
See [differences.md](differences.md) for a summary of notable behavioral differences.

| Option | Short | Supported | Parity | Default | Interactions | Tests | Notes | Enhanced? |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `--8-bit-output` | `-8` | ❌ | — | off | — | — |  |  |
| `--acls` | `-A` | ✅ | ❌ | off | requires `acl` feature | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) | requires `acl` feature |  |
| `--address` | — | ❌ | — | — | — | — |  |  |
| `--append` | — | ❌ | — | off | — | — |  |  |
| `--append-verify` | — | ❌ | — | off | — | — |  |  |
| `--archive` | `-a` | ✅ | ❌ | off | — | [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh) |  |  |
| `--atimes` | `-U` | ❌ | — | off | — | — |  |  |
| `--backup` | `-b` | ❌ | — | off | — | — |  |  |
| `--backup-dir` | — | ❌ | — | — | — | — |  |  |
| `--block-size` | `-B` | ❌ | — | — | — | — |  |  |
| `--blocking-io` | — | ❌ | — | off | — | — |  |  |
| `--bwlimit` | — | ✅ | ❌ | — | — | [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs) |  |  |
| `--cc` | — | ❌ | — | off | alias for `--checksum-choice` | [gaps.md](gaps.md) | alias for `--checksum-choice` |  |
| `--checksum` | `-c` | ✅ | ✅ | off | — | [tests/cli.rs](../tests/cli.rs) | strong hashes: MD5 (default), SHA-1, BLAKE3 |  |
| `--checksum-choice` | — | ❌ | — | — | choose the checksum algorithm (aka --cc) | — |  |  |
| `--checksum-seed` | — | ❌ | — | — | — | — |  |  |
| `--chmod` | — | ❌ | — | — | — | — |  |  |
| `--chown` | — | ❌ | — | — | — | — |  |  |
| `--compare-dest` | — | ❌ | — | — | — | — |  |  |
| `--compress` | `-z` | ✅ | ✅ | off | — | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/compression_negotiation.sh](../tests/compression_negotiation.sh) | negotiates zstd when supported by both peers |  |
| `--compress-choice` | — | ❌ | — | — | choose the compression algorithm (aka --zc) | — |  |  |
| `--compress-level` | — | ✅ | ❌ | — | explicitly set compression level (aka --zl) | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) |  |  |
| `--zc` | — | ❌ | — | off | alias for `--compress-choice` | [gaps.md](gaps.md) | alias for `--compress-choice` |  |
| `--zl` | — | ❌ | — | off | alias for `--compress-level` | [gaps.md](gaps.md) | alias for `--compress-level` |  |
| `--contimeout` | — | ❌ | — | — | — | — |  |  |
| `--copy-as` | — | ❌ | — | — | — | — |  |  |
| `--copy-dest` | — | ❌ | — | — | — | — |  |  |
| `--copy-devices` | — | ❌ | — | off | — | — |  |  |
| `--copy-dirlinks` | `-k` | ❌ | — | off | — | — |  |  |
| `--copy-links` | `-L` | ❌ | — | off | — | — |  |  |
| `--copy-unsafe-links` | — | ❌ | — | off | — | — |  |  |
| `--crtimes` | `-N` | ❌ | — | off | — | — |  |  |
| `--cvs-exclude` | `-C` | ❌ | — | off | — | — |  |  |
| `--daemon` | — | ✅ | ❌ | off | — | [tests/daemon.rs](../tests/daemon.rs) |  |  |
| `--debug` | — | ❌ | — | — | — | — |  |  |
| `--del` | — | ✅ | ✅ | off | an alias for --delete-during | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | alias for `--delete-during` |  |
| `--delay-updates` | — | ❌ | — | off | — | — |  |  |
| `--delete` | — | ✅ | ✅ | off | — | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  |  |
| `--delete-after` | — | ✅ | ✅ | off | — | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  |  |
| `--delete-before` | — | ✅ | ✅ | off | — | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  |  |
| `--delete-delay` | — | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--delete-during` | — | ✅ | ✅ | off | — | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) |  |  |
| `--delete-excluded` | — | ❌ | — | off | — | — |  |  |
| `--delete-missing-args` | — | ❌ | — | off | — | — |  |  |
| `--devices` | — | ✅ | ❌ | off | — | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) |  |  |
| `--dirs` | `-d` | ❌ | — | off | — | — |  |  |
| `--dry-run` | `-n` | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--early-input` | — | ❌ | — | — | — | — |  |  |
| `--exclude` | — | ✅ | ❌ | — | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--exclude-from` | — | ✅ | ❌ | — | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--executability` | `-E` | ❌ | — | off | — | — |  |  |
| `--existing` | — | ❌ | — | off | — | — |  |  |
| `--fake-super` | — | ❌ | — | off | — | — |  |  |
| `--files-from` | — | ✅ | ❌ | — | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--filter` | `-f` | ✅ | ✅ | — | — | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  |  |
| `--force` | — | ❌ | — | off | — | — |  |  |
| `--from0` | `-0` | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--fsync` | — | ❌ | — | off | — | — |  |  |
| `--fuzzy` | `-y` | ❌ | — | off | — | — |  |  |
| `--group` | `-g` | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--groupmap` | — | ❌ | — | — | — | — |  |  |
| `--hard-links` | `-H` | ✅ | ❌ | off | — | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs) |  |  |
| `--help` | `-h (*)` | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--human-readable` | `-h` | ❌ | — | off | — | — |  |  |
| `--iconv` | — | ❌ | — | — | — | — |  |  |
| `--ignore-errors` | — | ❌ | — | off | — | — |  |  |
| `--ignore-existing` | — | ❌ | — | off | — | — |  |  |
| `--ignore-missing-args` | — | ❌ | — | off | — | — |  |  |
| `--ignore-times` | `-I` | ❌ | — | off | — | — |  |  |
| `--include` | — | ✅ | ❌ | — | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--include-from` | — | ✅ | ❌ | — | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--info` | — | ❌ | — | — | — | — |  |  |
| `--inplace` | — | ✅ | ✅ | off | — | [tests/golden/cli_parity/inplace.sh](../tests/golden/cli_parity/inplace.sh) |  |  |
| `--ipv4` | `-4` | ❌ | — | off | — | — |  |  |
| `--ipv6` | `-6` | ❌ | — | off | — | — |  |  |
| `--itemize-changes` | `-i` | ❌ | — | off | — | — |  |  |
| `--keep-dirlinks` | `-K` | ❌ | — | off | — | — |  |  |
| `--link-dest` | — | ❌ | — | — | — | — |  |  |
| `--links` | `-l` | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--list-only` | — | ❌ | — | off | — | — |  |  |
| `--log-file` | — | ❌ | — | — | — | — |  |  |
| `--log-file-format` | — | ❌ | — | — | — | — |  |  |
| `--max-alloc` | — | ❌ | — | — | — | — |  |  |
| `--max-delete` | — | ❌ | — | — | — | — |  |  |
| `--max-size` | — | ❌ | — | — | — | — |  |  |
| `--min-size` | — | ❌ | — | — | — | — |  |  |
| `--mkpath` | — | ❌ | — | off | — | — |  |  |
| `--modify-window` | `-@` | ❌ | — | — | — | — |  |  |
| `--munge-links` | — | ❌ | — | off | — | — |  |  |
| `--no-D` | — | ❌ | — | off | alias for `--no-devices --no-specials` | [gaps.md](gaps.md) | alias for `--no-devices --no-specials` |  |
| `--no-OPTION` | — | ❌ | — | off | — | — |  |  |
| `--no-implied-dirs` | — | ❌ | — | off | — | — |  |  |
| `--no-motd` | — | ✅ | ❌ | off | — | [tests/daemon.rs](../tests/daemon.rs) |  |  |
| `--numeric-ids` | — | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--old-args` | — | ❌ | — | off | — | — |  |  |
| `--old-d` | — | ❌ | — | off | alias for --old-dirs | [gaps.md](gaps.md) | alias for `--old-dirs` |  |
| `--old-dirs` | — | ❌ | — | off | — | — |  |  |
| `--omit-dir-times` | `-O` | ❌ | — | off | — | — |  |  |
| `--omit-link-times` | `-J` | ❌ | — | off | — | — |  |  |
| `--one-file-system` | `-x` | ❌ | — | off | — | — |  |  |
| `--only-write-batch` | — | ❌ | — | — | — | — |  |  |
| `--open-noatime` | — | ❌ | — | off | — | — |  |  |
| `--out-format` | — | ❌ | — | — | — | — |  |  |
| `--outbuf` | — | ❌ | — | — | — | — |  |  |
| `--owner` | `-o` | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--partial` | — | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--partial-dir` | — | ✅ | ❌ | — | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--password-file` | — | ✅ | ❌ | — | — | [tests/daemon.rs](../tests/daemon.rs) |  |  |
| `--perms` | `-p` | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--port` | — | ❌ | — | — | — | — |  |  |
| `--preallocate` | — | ❌ | — | off | — | — |  |  |
| `--progress` | — | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--protocol` | — | ❌ | — | — | — | — |  |  |
| `--prune-empty-dirs` | `-m` | ❌ | — | off | — | — |  |  |
| `--quiet` | `-q` | ✅ | ✅ | off | — | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  |  |
| `--read-batch` | — | ❌ | — | — | — | — |  |  |
| `--recursive` | `-r` | ✅ | ✅ | off | — | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  |  |
| `--relative` | `-R` | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--remote-option` | `-M` | ❌ | — | — | — | — |  |  |
| `--remove-source-files` | — | ❌ | — | off | — | — |  |  |
| `--rsh` | `-e` | ❌ | — | — | — | [tests/rsh.rs](../tests/rsh.rs) |  |  |
| `--rsync-path` | — | ❌ | — | — | — | [tests/rsync_path.rs](../tests/rsync_path.rs) |  |  |
| `--safe-links` | — | ❌ | — | off | — | — |  |  |
| `--secluded-args` | `-s` | ❌ | — | off | — | — |  |  |
| `--secrets-file` | — | ✅ | ❌ | off | — | [tests/daemon.rs](../tests/daemon.rs) |  |  |
| `--server` | — | ✅ | ❌ | off | — | [tests/server.rs](../tests/server.rs) | negotiates protocol version and codecs |  |
| `--size-only` | — | ❌ | — | off | — | — |  |  |
| `--skip-compress` | — | ❌ | — | — | — | — |  |  |
| `--sockopts` | — | ❌ | — | — | — | — |  |  |
| `--sparse` | `-S` | ✅ | ✅ | off | — | [tests/cli.rs](../tests/cli.rs) | creates holes for long zero runs |  |
| `--specials` | — | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--stats` | — | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--stderr` | — | ❌ | — | errors | — | — |  |  |
| `--stop-after` | — | ❌ | — | — | — | — |  |  |
| `--stop-at` | — | ❌ | — | — | — | — |  |  |
| `--suffix` | — | ❌ | — | ~ w/o --backup-dir | — | — |  |  |
| `--super` | — | ❌ | — | off | — | — |  |  |
| `--temp-dir` | `-T` | ❌ | — | — | — | — |  |  |
| `--timeout` | — | ❌ | — | — | — | — |  |  |
| `--times` | `-t` | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--trust-sender` | — | ❌ | — | off | — | — |  |  |
| `--update` | `-u` | ❌ | — | off | — | — |  |  |
| `--usermap` | — | ❌ | — | — | — | — |  |  |
| `--verbose` | `-v` | ✅ | ❌ | off | — | [tests/cli.rs](../tests/cli.rs) |  |  |
| `--version` | `-V` | ❌ | — | off | — | — |  |  |
| `--whole-file` | `-W` | ❌ | — | off | — | — |  |  |
| `--write-batch` | — | ❌ | — | — | — | — |  |  |
| `--write-devices` | — | ❌ | — | off | — | — |  |  |
| `--xattrs` | `-X` | ✅ | ❌ | off | requires `xattr` feature | [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs) | requires `xattr` feature |  |
