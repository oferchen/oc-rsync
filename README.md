# rsync-rs

rsync-rs is a modular reimplementation of the classic `rsync` utility in Rust. It develops the protocol, transport and synchronization engine as a collection of reusable crates and aims for eventual compatibility with existing rsync deployments while leveraging Rust's safety and concurrency strengths.

## Summary

**Mission**: Deliver fast, reliable file synchronization with eventual compatibility with the original `rsync`.

**Non‑negotiable constraints**: correctness, security, cross‑platform support, and open‑source dual licensing.

## Mission
- Deliver fast, reliable file synchronization.
- Maintain compatibility with existing rsync deployments.
- Provide a welcoming platform for contributors building the next generation of sync tooling.

## Non-negotiable Constraints
- **Correctness**: Data integrity is paramount; transfers must faithfully mirror source files.
- **Security**: Safe defaults and careful parsing to prevent memory and protocol vulnerabilities.
- **Cross-platform**: Aim to support Linux, macOS, and Windows.
- **Open source**: Dual-licensed under MIT and Apache-2.0.

## Compatibility
Platform support status is tracked in the [compatibility matrix](docs/compat_matrix.md).

## In-Scope Features
- Local and remote file synchronization.
- Delta-transfer algorithm with rolling checksum.
- Progress reporting and dry-run mode.
- Modular crate design covering protocol, checksums, filters, file walking, compression, transport, and CLI components.

## Out-of-Scope Features
- Graphical user interfaces.
- Cloud-specific integration or storage backends.
- Scheduling/daemonization; external tools should orchestrate recurring jobs.

## CLI
Documentation for invoking the command line interface, available flags, and
configuration precedence lives in [docs/cli.md](docs/cli.md).

Quick links:

- [Usage](docs/cli.md#usage)
- [Flags](docs/cli.md#flags)
- [Configuration precedence](docs/cli.md#configuration-precedence)

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
- `transport` – abstracts local and remote I/O, multiplexing channels over SSH, TCP, or other transports.
- `cli` – exposes a user-facing command line built on top of the engine and transport layers.
- `fuzz` – houses fuzz targets that stress protocol and parser logic for robustness.

## Milestone Roadmap
1. **M1—Bootstrap** – repository builds; `walk` and `checksums` crates produce file signatures.
2. **M2—Transfer Engine** – `engine` performs block matching and delta encoding for local sync.
3. **M3—Networking** – `transport` enables SSH/TCP remotes with version negotiation.
4. **M4—CLI Parity** – CLI exposes core rsync flags with end-to-end tests passing.
5. **M5—Filters & Compression** – `filters` and `compress` crates integrate with engine and CLI.
6. **M6—Fuzzing & Performance** – fuzz targets run in CI; basic benchmarks meet performance goals.
7. **M7—Stabilization** – documentation complete, cross-platform builds green, and compatibility matrix up to date.

## Testing
Run the full test suite with:

```
cargo test
```

## Fuzzing
The project includes fuzz targets under `crates/fuzz`.
See [docs/fuzzing.md](docs/fuzzing.md) for instructions on installing the
tooling and running the fuzzers.

## License
This project is dual-licensed under the terms of the [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE) licenses.
