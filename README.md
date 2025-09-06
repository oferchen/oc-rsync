# oc-rsync - Pure-Rust rsync (compatible with rsync 3.4.1 / protocol 32)

[![CI](https://github.com/oferchen/oc-rsync/actions/workflows/ci.yml/badge.svg)](https://github.com/oferchen/oc-rsync/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/oferchen/oc-rsync)](https://github.com/oferchen/oc-rsync/releases)

## Project statement

oc-rsync is a re‑implementation of rsync’s behavior in Rust, created by Ofer Chen (2025). This project is unaffiliated with the Rsync Samba team.

## Compatibility

- Targets upstream rsync 3.4.1 (protocol 32). See [docs/UPSTREAM.md](docs/UPSTREAM.md) for negotiated versions.
- Feature parity is tracked in the [feature matrix](docs/feature_matrix.md).
- Platform status and interoperability notes live in [docs/compat_matrix.md](docs/compat_matrix.md) and [docs/compatibility.md](docs/compatibility.md).
- Known gaps are cataloged in [docs/gaps.md](docs/gaps.md).

## Install

Prebuilt binaries and packages are published on [GitHub Releases](https://github.com/oferchen/oc-rsync/releases).

### Packages

```bash
# Debian/Ubuntu
sudo dpkg -i oc-rsync_<version>_amd64.deb

# Fedora/RHEL
sudo rpm -i oc-rsync-<version>.x86_64.rpm

# Homebrew
brew install --build-from-source ./packaging/brew/oc-rsync.rb
```

### From source

#### Prerequisites

This project requires **Rust 1.87** or newer. The source build also needs a C toolchain, compression libraries, and ACL headers:

- `build-essential` (provides `gcc`/`ld`)
- `libzstd-dev`
- `zlib1g-dev`
- `libacl1-dev`

On Debian/Ubuntu systems:

```bash
sudo apt-get update
sudo apt-get install -y build-essential libzstd-dev zlib1g-dev libacl1-dev
```

Run `scripts/preflight.sh` to verify these dependencies before building. Then
install from crates.io or build from a local checkout:

If `libacl1-dev` isn't available, disable ACL support with

```bash
cargo build --no-default-features --features xattr
```

```bash
# install from crates.io
cargo install oc-rsync

# build and install from a local checkout
cargo install --path .

# or just compile
cargo build --bin oc-rsync
```

See [packaging/](packaging) for daemon configs and systemd units.

## Usage

```bash
$ oc-rsync --help
oc-rsync 0.1.0 — Pure-Rust rsync replica (compatible with rsync 3.4.1 / protocol 32)
...
```

The CLI aims for byte-for-byte parity with upstream `rsync`. Additional examples and flag descriptions are in [docs/cli.md](docs/cli.md), and any divergences from upstream are tracked in [docs/gaps.md](docs/gaps.md).

## Design/Architecture

An overview of crate boundaries, data flow, and algorithms is in [docs/architecture.md](docs/architecture.md). Agent responsibilities and protocol constants are detailed in [docs/UPSTREAM.md](docs/UPSTREAM.md), [docs/transport.md](docs/transport.md), and [docs/filters.md](docs/filters.md).

## Testing & CI

`make test` runs the full test suite with `cargo nextest run --workspace --no-fail-fast` followed by `cargo nextest run --workspace --no-fail-fast --features "cli nightly"`. CI workflows live under [.github/workflows](.github/workflows). Fuzzing and interoperability grids are described in [docs/fuzzing.md](docs/fuzzing.md) and [docs/interop-grid.md](docs/interop-grid.md).

## Security

Report vulnerabilities via [SECURITY.md](SECURITY.md). The daemon aims to match upstream hardening and privilege‑drop semantics.

## Roadmap

- M1-Bootstrap: repository builds; walk and checksum crates generate file signatures.
- M2-Delta Engine: local delta transfers with metadata preservation.
- M3-Remote Protocol: rsync protocol v32 over SSH and `rsync://`.
- M4-Metadata Fidelity: permissions, symlinks, hard links, sparse files, xattrs/ACLs.
- M5-Filters & Compression: include/exclude rules and compression negotiation.
- M6-Robust Transfers: resume partials, verify checksums, hardened I/O.
- M7-Stabilization: cross‑platform builds, performance tuning, and compatibility matrix completion.

## License

Dual-licensed under Apache-2.0 and MIT terms. See [LICENSE](LICENSE) for details.

## Acknowledgements

Inspired by the original [rsync](https://rsync.samba.org/) by Andrew Tridgell and the Samba team. Thanks to the Rust community and upstream projects that made this re‑implementation possible.

