# oc-rsync

oc-rsync is a modular reimplementation of the classic `rsync` utility in Rust. It develops the protocol, transport and synchronization engine as a collection of reusable crates and aims for eventual compatibility with existing rsync deployments while leveraging Rust's safety and concurrency strengths.

## Summary

**Mission**: Implement a pure-Rust rsync replacement compatible with stock rsync v32 over SSH and `rsync://`, with optional support for modern mode 73.

**Non‑negotiable constraints**: correctness with full metadata fidelity, security, robust I/O with resumable transfers, cross‑platform support, and open‑source dual licensing.

## Mission
- Implement a pure-Rust rsync replacement compatible with stock rsync v32 over SSH and `rsync://`, supporting modern mode 73.
- Deliver fast, reliable file synchronization.
- Provide a welcoming platform for contributors building the next generation of sync tooling.

## Non-negotiable Constraints
- **Correctness**: Transfers must faithfully mirror source files and metadata, preserving permissions, timestamps, ownership, symlinks, hard links, sparse files, and xattrs/ACLs.
- **Security**: Safe defaults and careful parsing to prevent memory and protocol vulnerabilities.
- **Cross-platform**: Aim to support Linux, macOS, and Windows.
- **Robust I/O**: Resume partial transfers and recover cleanly from interruptions.
- **Open source**: Dual-licensed under MIT and Apache-2.0.

## Compatibility
Platform support status is tracked in the [compatibility matrix](docs/compat_matrix.md).
An overview of tested operating systems and interoperability notes lives in
[docs/compatibility.md](docs/compatibility.md). Known gaps are documented in
[docs/gaps.md](docs/gaps.md), and a detailed feature breakdown is available in
[docs/feature_matrix.md](docs/feature_matrix.md).

## Specifications
The upstream rsync(1) and rsyncd.conf(5) man pages from rsync 3.4.x are bundled in [docs/spec](docs/spec) and serve as the project's authoritative specification. See [docs/spec/rsync.1](docs/spec/rsync.1) and [docs/spec/rsyncd.conf.5](docs/spec/rsyncd.conf.5).

## In-Scope Features
- Local and remote file synchronization over SSH and `rsync://`.
- Delta-transfer algorithm with rolling checksum.
- Preservation of permissions, symlinks, hard links, sparse files, extended attributes, and ACLs.
- Flexible include/exclude filtering.
- Compression negotiation between peers.
- Resume of interrupted transfers and handling of partial files.
- Robust I/O and durable writes.
- Progress reporting and dry-run mode.
- Modular crate design covering protocol, checksums, filters, file walking, compression, transport, and CLI components.

## Out-of-Scope Features
- Graphical user interfaces.
- Cloud-specific integration or storage backends.
- Scheduling/daemonization; external tools should orchestrate recurring jobs.

## CLI
Documentation for invoking the command line interface, available flags, and
configuration precedence lives in [docs/cli.md](docs/cli.md). Differences from
classic `rsync` are covered in [docs/differences.md](docs/differences.md).

Quick links:

- [Usage](docs/cli.md#usage)
- [Flags](docs/cli.md#flags)
- [Configuration precedence](docs/cli.md#configuration-precedence)

### Logging

Enable structured JSON logs on the command line with `--log-format json`. Library
callers can configure logging by building a [`SyncConfig`] with
`log_format: LogFormat::Json` and passing it to [`synchronize`].

## Architecture
See [docs/architecture.md](docs/architecture.md) for a deeper overview of crate
boundaries, data flow, and key algorithms.

Quick links:

- [Crate boundaries](docs/architecture.md#crate-boundaries)
- [Data flow](docs/architecture.md#data-flow)
- [Key algorithms](docs/architecture.md#key-algorithms)

The project is organized as a set of focused crates:

- `protocol` – defines frame formats, negotiates versions, and encodes/decodes messages on the wire.
- `checksums` – implements rolling and strong checksum algorithms for block matching.
- `filters` – parses include/exclude rules that control which files participate in a transfer.
- `walk` – traverses directories and produces the file list handed to the engine.
- `meta` – models file metadata (permissions, timestamps, ownership) and provides helper utilities.
- `compress` – offers traits and implementations for optional compression of file data during transfer.
- `engine` – orchestrates scanning, delta calculation, and application of differences between sender and receiver.
- `oc-rsync` – convenience wrapper around the engine that copies any files the
  engine skips during synchronization.
- `transport` – abstracts local and remote I/O, multiplexing channels over SSH, TCP, or other transports.
- `oc-rsync-cli` – exposes a user-facing command line built on top of the engine and transport layers.
- `fuzz` – houses fuzz targets that stress protocol and parser logic for robustness.

## Building

The CLI entry point resides in the `bin/oc-rsync` crate. Build or run it with Cargo:

```
cargo build -p oc-rsync-bin --bin oc-rsync
# or
cargo run -p oc-rsync-bin -- "<SRC>" "<DEST>"
```

## Packaging

Release tarballs include sample files under `packaging/` to help run the
daemon:

- `packaging/rsyncd.conf.example` – example configuration file
- `packaging/systemd/oc-rsyncd.service` – systemd service unit

## Milestone Roadmap
1. **M1—Bootstrap** – repository builds; `walk` and `checksums` crates generate file signatures.
2. **M2—Delta Engine** – `engine` drives local delta transfers with metadata preservation.
3. **M3—Remote Protocol** – rsync protocol v32 over SSH and `rsync://` implemented with modern mode 73 support.
4. **M4—Metadata Fidelity** – permissions, symlinks, hard links, sparse files, and xattrs/ACLs handled.
5. **M5—Filters & Compression** – include/exclude rules and compression negotiation wired through engine and CLI.
6. **M6—Robust Transfers** – resume partials, verify checksums, and harden I/O against interruptions.
7. **M7—Stabilization** – cross-platform builds, performance tuning, documentation, and compatibility matrix complete.

## Testing
Run the full test suite with:

```
make test
```

## Development

The Makefile offers shortcuts for common tasks. Repository guidelines require verifying file header comments:

```
make fmt    # Format the code
make clippy # Run Clippy lints
make doc    # Build documentation
make test   # Run the test suite
make verify-comments  # Validate file header comments
```

## Fuzzing
The project includes fuzz targets under `fuzz`.
See [docs/fuzzing.md](docs/fuzzing.md) for instructions on installing the
tooling and running the fuzzers.

## Performance
Performance benchmarks and CPU feature detection details are documented in
[docs/perf.md](docs/perf.md).

## Manifest

Content-defined chunking persists a manifest of seen chunks to
`~/.oc-rsync/manifest`. Users upgrading from earlier releases should
manually move any existing manifest from `~/.rsync-rs/manifest` to this
path.

## License
This project is dual-licensed under the terms of the [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE) licenses.
