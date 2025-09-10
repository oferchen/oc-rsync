# Project Overview

## Specifications
The upstream rsync(1) and rsyncd.conf(5) man pages from rsync 3.4.1 are included under [spec/](spec) as authoritative references for expected behavior: [spec/rsync.1](spec/rsync.1) and [spec/rsyncd.conf.5](spec/rsyncd.conf.5).

## Crates
- **protocol**: core frame and message types for the rsync protocol.
- **checksums**: rolling checksum and strong hash algorithms.
- **filelist**: encode and decode file lists for delta transfers.
- **filters**: include/exclude rule parsing and evaluation.
- **walk**: directory traversal utilities.
- **meta**: serialization and application of file metadata such as permissions, timestamps, and optional xattrs/ACLs.
- **compress**: compression codecs and negotiation helpers.
- **engine**: delta-transfer and synchronization engine.
- **transport**: blocking transport implementations including SSH and TCP.
- **daemon**: rsync:// server implementation.
- **cli**: command-line interface driving client, daemon, and probe modes.
- **logging**: structured logging utilities.
- **oc-rsync (root library)**: convenience wrapper around the engine's synchronization that
  delegates to `engine::sync` and falls back to a basic recursive copy only for
  files the engine does not yet handle.
- **fuzz**: fuzz targets validating protocol and filter robustness.

## Binaries
- **oc-rsync** (binary target): main CLI entry point.
- **protocol_frame_decode_fuzz**: fuzz target exercising protocol frame decoding.
- **filters_parse_fuzz**: fuzz target exercising filter rule parsing.

## Milestones
- [x] M1—Bootstrap
  - [x] Directory traversal and checksum generation
  - [x] Local sync via CLI
- [ ] M2—Transfer Engine
  - [x] Delta-transfer engine
  - [ ] Resume/partial transfers *(planned)*
- [x] M3—Networking
  - [x] Network transport (SSH/TCP)
- [ ] M4—CLI Parity
  - [ ] Permissions (ownership, timestamps, modes) *(in progress)*
  - [ ] Symlink preservation *(in progress)*
- [x] M5—Filters & Compression
  - [x] Include/exclude filters
  - [x] Compression negotiation
- [ ] M7—Stabilization
  - [ ] Hardlink support *(planned)*
  - [ ] Sparse file handling *(planned)*
  - [x] Extended attributes (xattrs) & ACLs

## Interoperability
For compatibility tracking with other implementations, see the [compatibility matrix](compat_matrix.md).
