# rsync-rs

rsync-rs is a modular reimplementation of the classic `rsync` utility in Rust. It develops the protocol, transport and synchronization engine as a collection of reusable crates and aims for eventual compatibility with existing rsync deployments while leveraging Rust's safety and concurrency strengths.

## Mission
- Deliver fast, reliable file synchronization.
- Maintain compatibility with existing rsync deployments.
- Provide a welcoming platform for contributors building the next generation of sync tooling.

## Non-negotiable Constraints
- **Correctness**: Data integrity is paramount; transfers must faithfully mirror source files.
- **Security**: Safe defaults and careful parsing to prevent memory and protocol vulnerabilities.
- **Cross-platform**: Aim to support Linux, macOS, and Windows.
- **Open source**: Dual-licensed under MIT and Apache-2.0.

## In-Scope Features
- Local and remote file synchronization.
- Delta-transfer algorithm with rolling checksum.
- Progress reporting and dry-run mode.
- Modular crate design covering protocol, checksums, filters, file walking, compression, transport, and CLI components.

## Out-of-Scope Features
- Graphical user interfaces.
- Cloud-specific integration or storage backends.
- Scheduling/daemonization; external tools should orchestrate recurring jobs.

## Crate Layout
- `protocol`: wire-level message encoding and negotiation.
- `checksums`: rolling and strong checksum algorithms.
- `filters`: include/exclude rule parsing.
- `walk`: directory traversal and file list generation.
- `meta`: file metadata types and helpers.
- `compress`: compression traits and implementations.
- `engine`: synchronization engine orchestrating delta transfer.
- `transport`: I/O abstraction for local and remote transports.
- `cli`: command-line interface and argument parsing.
- `fuzz`: fuzz testing targets for parser and protocol components.

## Milestone Roadmap
1. **Bootstrap** – basic file scanning and checksum generation.
2. **Transfer Engine** – implement block matching and delta encoding.
3. **Networking** – add remote transport over SSH and TCP.
4. **CLI Parity** – mirror key rsync flags and behaviors.
5. **Stabilization** – cross-platform polish and documentation.

## Testing
Run the full test suite with:

```
cargo test
```

## Fuzzing
The project includes fuzz targets under `crates/fuzz`. To run them, install [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) and execute:

```
cargo install cargo-fuzz
cd crates/fuzz
cargo fuzz run protocol_frame_decode_fuzz
cargo fuzz run filters_parse_fuzz
```
