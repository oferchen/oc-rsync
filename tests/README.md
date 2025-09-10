# Tests

Some tests require elevated privileges or network access. These are gated behind
Cargo features and are ignored by default.

- `root`: tests that need root privileges or `CAP_CHOWN`. Run with
  `cargo nextest run --features root -- --ignored` to execute.
- `network`: tests that start network services or perform network I/O. Run with
  `cargo nextest run --features network -- --ignored` to execute.

To run the full suite including these tests, combine the features as needed:

```bash
cargo nextest run --features "root network" -- --ignored
```
