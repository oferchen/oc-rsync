# Differences from rsync

`rsync-rs` strives to mirror the traditional `rsync` command line. The table
below highlights how common flags map to current support and how the
`--modern` convenience flag influences behavior. For a complete listing see
[cli/flags.md](cli/flags.md).

| rsync flag | rsync-rs status | `--modern` notes |
|------------|-----------------|------------------|
| `-z`, `--compress` | ✅ uses zlib by default | negotiates zstd if both peers support it |
| `--compress-level` | ✅ maps numeric levels | applies to zlib or zstd |
| `-c`, `--checksum` | ❌ parsed but not implemented | would switch to BLAKE3 when available |
| `-a`, `--archive` | ✅ sets perms, times, owner, group, links, devices, specials | n/a |
| `-R`, `--relative` | ✅ preserves ancestor directories | n/a |
| `-P` | ✅ keeps partial files and shows progress | n/a |
| `--numeric-ids` | ✅ uses numeric uid/gid values | n/a |
| `--modern` | rsync-rs only | enables zstd compression and BLAKE3 checksums |

## Additional notes

- `--daemon` and `--server` have the same syntax and defaults as `rsync`; see [cli.md](cli.md#daemon-and-server-modes).
- `-e`/`--rsh` defaults to `ssh` and honors the `RSYNC_RSH` environment variable; see [cli.md](cli.md#remote-shell).
- Only `--delete` is currently supported among deletion flags. Other variants are
  parsed but not implemented. See [cli.md](cli.md#deletion-flags).
- Advanced transfer options such as `--partial`, `--bwlimit`, and `--link-dest`
  behave like `rsync` when available; see
  [cli.md](cli.md#advanced-transfer-options).

