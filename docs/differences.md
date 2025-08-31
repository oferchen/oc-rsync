# Differences from rsync

oc-rsync implements the standard rsync protocol version 32 and also defines a
private protocol 73 extension used only when both peers are oc-rsync.

See [gaps.md](gaps.md) and [feature_matrix.md](feature_matrix.md) for any remaining parity notes.

## Protocol 73 features

- BLAKE3 strong checksums negotiated via `--modern` or `--modern-hash`.
- zstd and lz4 compression via `--modern` or `--modern-compress`.
- Optional FastCDC chunking with `--modern-cdc`.

## Manifest location

The content-defined chunking manifest now resides at `~/.oc-rsync/manifest`.
Previous versions stored it at `~/.rsync-rs/manifest`. To continue using an
existing manifest, move it to the new location.
