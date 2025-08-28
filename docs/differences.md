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

