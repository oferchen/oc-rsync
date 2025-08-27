# Architecture Overview

This document expands on the [README architecture section](../README.md#architecture)
with a deeper look at crate boundaries, data flow, and the key algorithms that
power `rsync-rs`.

## Crate boundaries

- `protocol` – defines frame formats, negotiates versions, and encodes/decodes
  messages on the wire.
- `transport` – abstracts local and remote I/O, multiplexing channels over SSH,
  TCP, or other transports.
- `walk` – traverses directories and produces the file list fed into the engine.
- `checksums` – computes rolling and strong checksums for block matching.
- `engine` – orchestrates scanning, delta calculation, and application of
  differences between sender and receiver.
- `compress` – offers optional compression of file data during transfer.
- `filters` – parses include/exclude rules controlling which files participate
  in a transfer.
- `meta` – models file metadata (permissions, timestamps, ownership) and
  provides helper utilities.
- `cli` – exposes a user-facing command line built on top of the engine and
  transport layers.
- `fuzz` – houses fuzz targets that stress protocol and parser logic for
  robustness.

## Data flow

1. The `walk` crate scans the filesystem and yields metadata for each file.
2. `checksums` computes rolling and strong checksums for file blocks.
3. The `engine` compares source and destination states using those checksums and
   produces a delta describing the necessary changes.
4. `protocol` frames the delta into messages and `transport` carries them over
   the chosen medium.
5. The receiver applies updates to its local filesystem, completing the sync
   cycle.

## Key algorithms

- **Rolling checksum**: a fast, Adler-32–style checksum enables block matching
  without reading the entire file.
- **Strong checksum**: cryptographic hashes (e.g., BLAKE3) verify block
  identity, reducing collision risk.
- **Delta encoding**: only modified blocks are transmitted, minimizing
  bandwidth and disk I/O.

Together these pieces form a pipeline that mirrors classic `rsync` while
leveraging Rust's safety and concurrency strengths.
