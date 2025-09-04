# Failing test report

The command-line parser now accepts positional `SRC`/`DST` paths alongside options, resolving prior usage errors. The current test run fails during linking because the system lacks `libacl`.

## Default features

- `cargo nextest run --workspace --no-fail-fast` fails to link: `/usr/bin/ld: cannot find -lacl`.
- No test assertions were executed.

## Feature-gated runs

- `--features acl`: fails to link for the same missing `libacl` dependency.

## Other checks

- `make verify-comments` reports `tests/specials_parity.rs: incorrect header`.
- `make lint` (cargo fmt --all --check) passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.

No relevant upstream issues or pull requests were found.
