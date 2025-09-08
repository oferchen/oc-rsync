# Architecture Overview

This document expands on the [README architecture section](../README.md#architecture)
and the [in-scope features](../README.md#in-scope-features) with a deeper look at
crate boundaries, data flow, and the key algorithms that power `oc-rsync`.

## Crates and module hierarchy

### [`protocol`](../crates/protocol)

- `mux` and `demux` split and join channel traffic.
- `server` drives version negotiation and framing.
- **Design patterns**: builder for handshake settings; factory for message
  creation.

### [`transport`](../crates/transport)

- `ssh` and `tcp` modules provide concrete transports with optional `rate`
  limiting.
- **Design patterns**: strategy pattern selects the transport, while a small
  factory instantiates the chosen backend.

### [`walk`](../crates/walk)

- Single `lib` module exposing a filesystem iterator.
- **Design patterns**: a lightweight builder configures traversal options.

### [`checksums`](../crates/checksums)

- Houses rolling and strong checksum logic.
- `ChecksumConfigBuilder` constructs settings and a strategy dispatches between
  supported algorithms.
- **Design patterns**: builder and strategy.

### [`engine`](../crates/engine)

- Modules: `batch`, `block`, `cleanup`, `delta`, `flist`, `io`, `receiver`,
  `sender`, and `session`.
- **Design patterns**: factory methods assemble sender/receiver pieces.

### [`compress`](../crates/compress)

- Single module selecting compression algorithms.
- **Design patterns**: strategy for codec choice.

### [`filters`](../crates/filters)

- Modules: `parser`, `rule`, `matcher`, `perdir`, and `stats`.
- **Design patterns**: strategy governs rule evaluation.

### [`filelist`](../crates/filelist)

- Encodes and decodes file lists exchanged over the wire.
- **Design patterns**: factory routines build entries as data is streamed.

### [`meta`](../crates/meta)

- Modules: `parse`, `unix`, `windows`, and `stub` for platform-specific
  metadata handling.
- **Design patterns**: builder assembles preservation options.

### [`daemon`](../crates/daemon)

- Runs the rsync-style daemon with module configuration via a builder.

### [`logging`](../crates/logging)

- `lib` wires structured logging; `formatter` tweaks output styles.
- **Design patterns**: observer pattern fans logs out to multiple sinks.

### [`oc-rsync-cli`](../crates/cli)

- Modules: `options`, `session`, `exec`, `daemon`, `formatter`, `utils`,
  `version`, and `branding`.
- **Design patterns**: builder validates user options and a small factory
  prepares sessions.

### [`fuzz`](../fuzz)

- Contains fuzz targets exercising protocol and parser logic for parity.

## Version and capability negotiation

Every session begins with a handshake where both peers advertise the highest
protocol version they support along with a bitmask of optional capabilities.
oc-rsync implements the upstream rsync protocol version 32 and defines a
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
  (e.g., SHA-1) verify block identity, reducing collision risk.
- **Delta encoding** ([`engine`](../crates/engine)): only modified blocks are
  transmitted, minimizing bandwidth and disk I/O.

Together these pieces form a pipeline that mirrors classic `rsync` while
leveraging Rust's safety and concurrency strengths.
