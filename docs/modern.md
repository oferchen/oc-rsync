# Modern Mode

oc-rsync implements the upstream rsync protocol version 31 and adds a private
protocol 73 extension for optional features. Protocol 73 is negotiated when both
peers invoke `oc-rsync` with the `--modern` flag or one of its sub-flags.

## Stronger checksums

`--modern` or `--modern-hash` upgrades the strong file checksum to BLAKE3 when
supported.  This provides better error detection than MD5 or SHA-1 while
remaining fast.

## Enhanced compression

Modern mode prefers zstd compression and also supports lz4 through the
`--modern-compress` option.  When both peers support the algorithm the most
efficient codec is selected automatically.

## Content-defined chunking

The `--modern-cdc` flag enables FastCDC chunking to improve delta reuse between
runs.  This is only activated when both sides negotiate protocol 73.

All modern features require both peers to speak protocol 73; transfers fall back
to stock rsync semantics when a peer does not advertise support.
