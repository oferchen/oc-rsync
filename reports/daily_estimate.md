# Daily Estimate

This report captures snapshot metrics for the project.

## Current metrics (2025-09-03)
- **Test pass rate:** unit tests 9/9; some integration tests currently failing.
- **Coverage:** not collected (run `make coverage` after installing `cargo-llvm-cov`).
- **Build time:** 1m16s for `cargo build` on a clean tree.

## Methodology
- *Test pass rate* counts passing tests from `cargo test`.
- *Coverage* is produced by `make coverage`, which invokes `cargo llvm-cov` and writes `reports/coverage.json`.
- *Build time* measures wall-clock time from `cargo clean && time cargo build`.

## Updating
To refresh these numbers:
```bash
cargo clean && time cargo build
cargo test
make coverage
```
Then edit this file with the new values. CI can run the same commands and commit the result automatically.
