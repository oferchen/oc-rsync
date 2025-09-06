# Contributing

Thank you for your interest in contributing to oc_rsync!

## Coding Standards
- Format code with `cargo fmt --all`.
- Lint with `cargo clippy --all-targets --all-features -- -D warnings` before committing.
- Keep contributions focused and document any new functionality.
- Use the workspace `Cargo.lock` at the repository root; do not commit lockfiles in individual crates.
- All `*.rs` files must begin with a single comment containing their relative path (e.g. `// src/lib.rs`) and contain no other comments, as enforced in `AGENTS.md`. Validate with `scripts/check-comments.sh` or the canonical `make verify-comments` check.

## Makefile targets

The Makefile offers shortcuts for common CI checks:

- `make verify-comments` – canonical check that runs `scripts/check-comments.sh` to enforce comment headers.
- `make lint` – run `cargo fmt --all --check` for formatting.
- `make test` – run `cargo nextest run --workspace --no-fail-fast` followed by
  `cargo nextest run --workspace --no-fail-fast --features "cli nightly"`.
- `make coverage` – execute `cargo llvm-cov nextest --workspace --features "cli nightly" --doctests \
  --fail-under-lines 95 --fail-under-functions 95` to gather test coverage.
- `make interop` – run the interoperability matrix with `tests/interop/run_matrix.sh`.
  These tests are behind the `interop` feature and require upstream `rsync`
  binaries.

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
- Install `cargo-nextest` with `cargo install cargo-nextest --locked` if it's not already installed, or run `./scripts/install-nextest.sh` to verify or install it.
- Ensure `cargo nextest run --workspace --no-fail-fast` and
  `cargo nextest run --workspace --no-fail-fast --features "cli nightly"`
  pass locally. Interoperability tests in `tests/interop/` are gated behind the
  `interop` feature and require an upstream `rsync` binary. Run them with
  `cargo nextest run --workspace --no-fail-fast --features "interop" --run-ignored=only-tests`
  or invoke `make interop`.
- Add or update tests for any new code.
- Prefer small, focused commits that each pass the test suite.

## Standardized Test Environment

For deterministic CLI help and usage output, run tests with a consistent
environment:

```bash
LC_ALL=C LANG=C COLUMNS=80 TZ=UTC make test
```

The `make test` target exports these variables automatically and runs
`cargo nextest run --workspace --no-fail-fast` followed by
`cargo nextest run --workspace --no-fail-fast --features "cli nightly"`.

## Fetching upstream rsync
Some interop tests require the official rsync sources. Use the helper
script to download and verify the tarball:

```bash
./scripts/fetch-rsync.sh
```

The script checks the tarball's SHA256 digest and extracts it in the
repository root. Continuous integration runs this script automatically.
