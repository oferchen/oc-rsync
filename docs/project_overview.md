# Project Overview

## Specifications
The upstream rsync(1) and rsyncd.conf(5) man pages from rsync 3.4.x are included under [spec/](spec) as authoritative references for expected behavior: [spec/rsync.1](spec/rsync.1) and [spec/rsyncd.conf.5](spec/rsyncd.conf.5).

## Crates
- **protocol**: core frame and message types for the rsync protocol.
- **checksums**: rolling checksum and strong hash algorithms.
- **filters**: include/exclude rule parsing and evaluation.
- **walk**: directory traversal utilities.
- **meta**: serialization and application of file metadata such as permissions, timestamps, and optional xattrs/ACLs.
- **compress**: compression codecs and negotiation helpers.
- **engine**: delta-transfer and synchronization engine.
- **transport**: blocking transport implementations including SSH and TCP.
- **oc-rsync-cli**: command-line interface driving client, daemon, and probe modes.
- **oc-rsync (root library)**: convenience wrapper around the engine's synchronization.
- **fuzz**: fuzz targets validating protocol and filter robustness.

## Binaries
- **oc-rsync** (`bin/oc-rsync` crate): main CLI entry point.
- **protocol_frame_decode_fuzz**: fuzz target exercising protocol frame decoding.
- **filters_parse_fuzz**: fuzz target exercising filter rule parsing.

## Milestones
- [x] M1—Bootstrap
  - [x] Directory traversal and checksum generation
  - [x] Local sync via CLI
- [ ] M2—Transfer Engine
  - [ ] Delta-transfer engine *(in progress)*
  - [ ] Resume/partial transfers *(planned)*
- [ ] M3—Networking
  - [ ] Network transport (SSH/TCP) *(in progress)*
- [ ] M4—CLI Parity
  - [ ] Permissions (ownership, timestamps, modes) *(in progress)*
  - [ ] Symlink preservation *(in progress)*
- [ ] M5—Filters & Compression
  - [ ] Include/exclude filters *(in progress)*
  - [ ] Compression negotiation *(in progress)*
- [ ] M7—Stabilization
  - [ ] Hardlink support *(planned)*
  - [ ] Sparse file handling *(planned)*
  - [x] Extended attributes (xattrs) & ACLs

## Interoperability
For compatibility tracking with other implementations, see the [compatibility matrix](compat_matrix.md).
