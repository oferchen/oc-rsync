# Prework Report

## `cargo nextest run --workspace --no-fail-fast`
- **Status:** Failed
- **Details:** Linker error: `/usr/bin/ld: cannot find -lacl`
- **Impacted tests:** `engine` crate tests (e.g., `streaming`), `oc-rsync-bin` binary tests
- **Applied fixes:** None
- **Remaining risks:** Missing ACL development library blocks building and testing

## `cargo nextest run --workspace --all-features --no-fail-fast`
- **Status:** Failed
- **Details:** Linker error: `/usr/bin/ld: cannot find -lacl`
- **Impacted components:** `protocol` crate tests
- **Applied fixes:** None
- **Remaining risks:** Same as above; cannot exercise all features without ACL library

## `make verify-comments`
- **Status:** Failed
- **Details:** `tests/specials_parity.rs` reports `incorrect header`
- **Applied fixes:** None
- **Remaining risks:** Comment formatting check fails until headers corrected

## `cargo clippy --all-targets --all-features -- -D warnings`
- **Status:** Passed

## `make lint`
- **Status:** Passed (`cargo fmt --all --check`)
