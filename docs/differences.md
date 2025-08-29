# Differences from rsync

`rsync-rs` aims for parity with stock `rsync` 3.2.x. When run without the
`--modern` flag, it intends zero behavioral differences from the traditional
utility and mirrors the classic command line.

The `--modern` convenience flag enables additional enhancements beyond
classic `rsync` behavior. For a complete listing see
[cli/flags.md](cli/flags.md). For detailed parity status see
[feature_matrix.md](feature_matrix.md).

## `--modern` enhancements

| rsync flag | rsync-rs status | Tests | `--modern` notes |
|------------|-----------------|-------|------------------|
| `-z`, `--compress` | ✅ uses zlib by default | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh) | negotiates zstd if both peers support it |
| `--compress-choice` | ✅ choose zstd or zlib | [tests/golden/cli_parity/compress-choice.sh](../tests/golden/cli_parity/compress-choice.sh) | n/a |
| `--compress-level` | ✅ maps numeric levels | [tests/golden/cli_parity/compress-level.sh](../tests/golden/cli_parity/compress-level.sh) | applies to zlib or zstd |
| `-c`, `--checksum` | ✅ strong hashes: MD5 (default), SHA-1, BLAKE3 | [tests/cli.rs](../tests/cli.rs) | `--modern` selects BLAKE3 |
| `-a`, `--archive` | ✅ sets perms, times, owner, group, links, devices, specials | [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh) | n/a |
| `-R`, `--relative` | ✅ preserves ancestor directories | [tests/cli.rs](../tests/cli.rs) | n/a |
| `-P` | ✅ keeps partial files and shows progress | [tests/cli.rs](../tests/cli.rs) | n/a |
| `--numeric-ids` | ✅ uses numeric uid/gid values | [tests/cli.rs](../tests/cli.rs) | n/a |
| `--modern` | rsync-rs only | [tests/interop/modern.rs](../tests/interop/modern.rs) | enables zstd compression and BLAKE3 checksums |

## Additional notes

- `--daemon` and `--server` have the same syntax and defaults as `rsync`; see [cli.md](cli.md#daemon-and-server-modes).
- `-e`/`--rsh` defaults to `ssh` and honors the `RSYNC_RSH` environment variable; see [cli.md](cli.md#remote-shell).
- Deletion flags `--delete-before`, `--delete-during`, `--delete-delay`,
  `--delete-after`, and `--delete-excluded` are implemented. See
  [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)
  for parity coverage and [feature_matrix.md](feature_matrix.md) for details.
- Advanced transfer options such as `--partial`, `--bwlimit`, and `--link-dest`
  behave like `rsync` when available; see
  [tests/cli.rs](../tests/cli.rs),
  [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs), and
  [tests/link_copy_compare_dest.rs](../tests/link_copy_compare_dest.rs).

