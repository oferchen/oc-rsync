# Coverage Exclusions

This document lists code that is intentionally excluded from the project's
coverage requirements. Coverage gates enforce a 95% threshold for both project
and patch metrics, as well as line and function coverage.

The following areas are excluded when interpreting coverage reports:

- `fuzz/` – fuzzing harnesses run with `cargo fuzz` live outside the workspace
  and are not included in coverage metrics.
- Shell-based integration and golden tests in `tests/` invoke external
  binaries and are not tracked by code coverage tools.
- Generated build scripts (`build.rs`) in workspace crates.

If additional code must be ignored by coverage, document it here to maintain
transparency around the enforced thresholds.
