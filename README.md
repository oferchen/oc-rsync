# oc-rsync — Pure-Rust rsync replica (compatible with rsync 3.4.1 / protocol 32)

[![CI](https://github.com/oc-rsync/oc-rsync/actions/workflows/ci.yml/badge.svg)](https://github.com/oc-rsync/oc-rsync/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/oc-rsync/oc-rsync/branch/main/graph/badge.svg)](https://codecov.io/gh/oc-rsync/oc-rsync)
[![Release](https://img.shields.io/github/v/release/oc-rsync/oc-rsync)](https://github.com/oc-rsync/oc-rsync/releases)

## Project statement

oc-rsync is an automatic re‑implementation of rsync’s behavior in Rust, created by Ofer Chen (2025). Not affiliated with the Samba project.

## Compatibility

- Targets upstream rsync 3.4.1 (protocol 32). See [docs/UPSTREAM.md](docs/UPSTREAM.md) for negotiated versions.
- Feature parity is tracked in the [feature matrix](docs/feature_matrix.md).
- Platform status and interoperability notes live in [docs/compat_matrix.md](docs/compat_matrix.md) and [docs/compatibility.md](docs/compatibility.md).
- Known gaps are cataloged in [docs/gaps.md](docs/gaps.md).

## Install

```bash
cargo install oc-rsync
# or build from source
cargo build -p oc-rsync-bin --bin oc-rsync
```

See [packaging/](packaging) for daemon configs and systemd units.

## Usage

```bash
$ oc-rsync --help
oc-rsync 0.1.0 — Pure-Rust rsync replica (compatible with rsync 3.4.1 / protocol 32)
...
```

The CLI aims for byte-for-byte parity with upstream `rsync`. Additional examples and flag descriptions are in [docs/cli.md](docs/cli.md) and upstream differences in [docs/differences.md](docs/differences.md).

## Design/Architecture

An overview of crate boundaries, data flow, and algorithms is in [docs/architecture.md](docs/architecture.md). Agent responsibilities and protocol constants are detailed in [docs/UPSTREAM.md](docs/UPSTREAM.md), [docs/transport.md](docs/transport.md), and [docs/filters.md](docs/filters.md).

## Testing & CI

`make test` runs the full test suite. CI workflows live under [.github/workflows](.github/workflows). Fuzzing and interoperability grids are described in [docs/fuzzing.md](docs/fuzzing.md) and [docs/interop-grid.md](docs/interop-grid.md).

## Security

Report vulnerabilities via [SECURITY.md](SECURITY.md). The daemon aims to match upstream hardening and privilege‑drop semantics.

## Roadmap

- M1—Bootstrap: repository builds; walk and checksum crates generate file signatures.
- M2—Delta Engine: local delta transfers with metadata preservation.
- M3—Remote Protocol: rsync protocol v32 over SSH and `rsync://`.
- M4—Metadata Fidelity: permissions, symlinks, hard links, sparse files, xattrs/ACLs.
- M5—Filters & Compression: include/exclude rules and compression negotiation.
- M6—Robust Transfers: resume partials, verify checksums, hardened I/O.
- M7—Stabilization: cross‑platform builds, performance tuning, and compatibility matrix completion.

## License

Dual-licensed under [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE).

## Acknowledgements

Inspired by the original [rsync](https://rsync.samba.org/) by Andrew Tridgell and the Samba team. Thanks to the Rust community and upstream projects that made this re‑implementation possible.

