# rsync-rs

rsync-rs aims to provide a Rust reimplementation of the classic `rsync` utility. The project seeks feature parity with the original while embracing Rust's safety and concurrency advantages.

## Mission
- Deliver fast, reliable file synchronization.
- Maintain compatibility with existing rsync deployments.
- Provide a welcoming platform for contributors building the next generation of sync tooling.

## Non-negotiable Constraints
- **Correctness**: Data integrity is paramount; transfers must faithfully mirror source files.
- **Security**: Safe defaults and careful parsing to prevent memory and protocol vulnerabilities.
- **Cross-platform**: Support Linux, macOS, and Windows from day one.
- **Open source**: Dual-licensed under MIT and Apache-2.0.

## In-Scope Features
- Local and remote file synchronization.
- Delta-transfer algorithm with rolling checksum.
- Progress reporting and dry-run mode.
- Extensible crate design for future transports and compressors.

## Out-of-Scope Features
- Graphical user interfaces.
- Cloud-specific integration or storage backends.
- Scheduling/daemonization; external tools should orchestrate recurring jobs.

## Crate Layout
- `rsync-core`: core algorithms and data structures.
- `rsync-cli`: command-line interface and argument parsing.
- `rsync-utils`: shared utilities and helper functionality.

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
