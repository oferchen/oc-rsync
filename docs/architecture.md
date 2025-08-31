# Architecture Overview

This document expands on the [README architecture section](../README.md#architecture)
and the [in-scope features](../README.md#in-scope-features) with a deeper look at
crate boundaries, data flow, and the key algorithms that power `oc-rsync`.

## Crate boundaries

- [`protocol`](../crates/protocol) – defines frame formats, negotiates versions,
  and encodes/decodes messages on the wire.
- [`transport`](../crates/transport) – abstracts local and remote I/O,
  multiplexing channels over SSH, TCP, or other transports.
- [`walk`](../crates/walk) – traverses directories and produces the file list
  fed into the engine.
- [`checksums`](../crates/checksums) – computes rolling and strong checksums for
  block matching.
- [`engine`](../crates/engine) – orchestrates scanning, delta calculation, and
  application of differences between sender and receiver. It also maintains a
  content-defined chunking manifest at `~/.oc-rsync/manifest`. Users upgrading
  from older versions should move any existing manifest from
  `~/.rsync-rs/manifest` to this path.
- [`compress`](../crates/compress) – offers optional compression of file data
  during transfer.
- [`filters`](../crates/filters) – parses include/exclude rules controlling
  which files participate in a transfer.
- [`meta`](../crates/meta) – models file metadata (permissions, timestamps,
  ownership) and provides helper utilities.
- [`oc-rsync-cli`](../crates/cli) – exposes a user-facing command line built on top of
  the engine and transport layers.
- [`fuzz`](../fuzz) – houses fuzz targets that stress protocol and parser
  logic for robustness.

### Transport

The [`transport`](../crates/transport) crate abstracts local and remote I/O.
It can spawn `ssh` in server mode (`--server` arguments) and capture the
process's stderr for diagnostics. Child stdio is wrapped in buffered readers and
writers to ensure bounded I/O when communicating with remote peers.

## Version and capability negotiation

Every session begins with a handshake where both peers advertise the highest
protocol version they support along with a bitmask of optional capabilities.
oc-rsync implements the upstream rsync protocol version 31 and defines a
private protocol 73 extension for oc-rsync-to-oc-rsync enhancements. The
`protocol` crate compares the peer's version with its own range of
[`MIN_VERSION`](../crates/protocol/src/lib.rs) through
[`LATEST_VERSION`](../crates/protocol/src/lib.rs) and selects the highest common
value. Capabilities are negotiated by intersecting the advertised bitmasks so
that both sides enable only features understood by each. If no overlap exists
the handshake fails with a negotiation error. Subsequent messages—including
keep-alives, progress updates and error reports—operate within the agreed
version and capability set.

## Data flow

1. The [`walk`](../crates/walk) crate scans the filesystem and yields metadata
   for each file.
2. [`checksums`](../crates/checksums) computes rolling and strong checksums for
   file blocks.
3. The [`engine`](../crates/engine) compares source and destination states using
   those checksums and produces a delta describing the necessary changes.
4. [`protocol`](../crates/protocol) frames the delta into messages and
   [`transport`](../crates/transport) carries them over the chosen medium.
5. The receiver applies updates to its local filesystem, completing the sync
   cycle.

## Key algorithms

- **Rolling checksum** ([`checksums`](../crates/checksums)): a fast,
  Adler-32–style checksum enables block matching without reading the entire
  file.
- **Strong checksum** ([`checksums`](../crates/checksums)): cryptographic hashes
  (e.g., BLAKE3) verify block identity, reducing collision risk.
- **Delta encoding** ([`engine`](../crates/engine)): only modified blocks are
  transmitted, minimizing bandwidth and disk I/O.

Together these pieces form a pipeline that mirrors classic `rsync` while
leveraging Rust's safety and concurrency strengths.
