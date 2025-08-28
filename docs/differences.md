# Differences from rsync

* Compression uses zlib by default. If both peers advertise support for zstd it will be selected automatically.
* `--compress-level` maps the provided numeric level to the underlying codec implementation.
* `--modern` enables zstd compression and BLAKE3 checksums on both peers.
