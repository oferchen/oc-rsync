# Prework Test Report

This document tracks the current state of failing tests in `oc-rsync` ahead of broader stabilization work. Update it whenever tests are fixed or new failures are introduced.

## Failing tests

- Test suite does not run under default configuration: `cargo test --workspace` fails during linking because the system `libacl` library is missing.
- Integration and CLI tests (e.g., `archive_matches_combination_and_rsync`, `checksum_forces_transfer_cli`) fail with usage errors when required positional arguments are rejected.

## Root causes

- Missing system dependency `libacl` prevents binaries from linking when ACL support is enabled.
- CLI argument parser is incomplete, causing valid invocations to be treated as missing required arguments.

## Remediation

- Install the required `libacl` development package on the build system so tests can link successfully.
- Implement remaining CLI options and positional argument handling to accept standard rsync invocations.

## Residual risks

- Additional tests may uncover further missing dependencies or unimplemented features once the current blockers are resolved.

> **Note:** Future contributors should update this file whenever they stabilize tests or introduce new failures.
