# Contributing

Thank you for your interest in contributing to rsync_rs!

## Coding Standards
- Format code with `cargo fmt --all`.
- Lint with `cargo clippy --all-targets --all-features -- -D warnings` before committing.
- Keep contributions focused and document any new functionality.

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
