# Contributing

Thank you for your interest in contributing to oc_rsync!

## Coding Standards
- Format code with `cargo fmt --all`.
- Lint with `cargo clippy --all-targets --all-features -- -D warnings` before committing.
- Keep contributions focused and document any new functionality.
- Use the workspace `Cargo.lock` at the repository root; do not commit lockfiles in individual crates.
- For wrapper Rust source files outside of `crates/` and `tests/`, begin the file with a comment containing its relative path, e.g. `// src/lib.rs`, and avoid any other comments. Run `scripts/check-comments.sh` to ensure compliance.

## Makefile targets

The Makefile offers shortcuts for common CI checks:

- `make verify-comments` – run `scripts/check-comments.sh` to enforce comment headers.
- `make lint` – run `cargo fmt --all --check` for formatting.
- `make coverage` – execute `cargo llvm-cov --workspace --doctests \
  --fail-under-lines 95 --fail-under-functions 95` to gather test coverage.
- `make interop` – run the interoperability matrix with `tests/interop/run_matrix.sh`.

## Continuous Integration

The CI workflow runs with a consistent environment and enforces comment
headers:

- Environment variables: `RUSTFLAGS="-Dwarnings"`, `LC_ALL=C`, `LANG=C`,
  `COLUMNS=80`, and `TZ=UTC`.
  - Repository access is read only (`permissions: { contents: read }`) and a
    concurrency group (`ci-${{ github.ref }}`) cancels in-progress runs for the same
    ref.
  - Builds cache dependencies using `Swatinem/rust-cache@v2`.
  - After `cargo clippy`, CI runs `make verify-comments` to ensure file header
  comments follow the policy.

## Pull Request Process
1. Fork the repository and create a topic branch.
2. Ensure your branch is up to date with the `main` branch.
3. Run formatting, linting, and tests.
4. Open a pull request describing your changes and reference any relevant issues.
5. A maintainer will review your PR and may request changes.

## Testing Requirements
- Ensure `cargo test` passes locally.
- Add or update tests for any new code.
- Prefer small, focused commits that each pass the test suite.

## Standardized Test Environment

For deterministic CLI help and usage output, run tests with a consistent
environment:

```bash
LC_ALL=C LANG=C COLUMNS=80 TZ=UTC make test
```

The `make test` target exports these variables automatically.

## Fetching upstream rsync
Some interop tests require the official rsync sources. Use the helper
script to download and verify the tarball:

```bash
./scripts/fetch-rsync.sh
```

The script checks the tarball's SHA256 digest and extracts it in the
repository root. Continuous integration runs this script automatically.
