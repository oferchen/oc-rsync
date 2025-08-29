# Fuzzing

This repository includes several fuzz targets under `fuzz`.
The harnesses are built with [`libFuzzer`](https://llvm.org/docs/LibFuzzer.html)
via the [`libfuzzer-sys`](https://crates.io/crates/libfuzzer-sys) crate.

Continuous integration runs extended fuzzing sessions for each target on every
pull request, and the job must pass before merging.

## Running a fuzzer

1. Install the tooling (once per machine):
   ```bash
   cargo install cargo-fuzz
   ```
2. Execute a target for a short period of time:
   ```bash
   cargo run -p fuzz --bin protocol_frame_decode_fuzz -- -max_total_time=30
   cargo run -p fuzz --bin filters_parse_fuzz -- -max_total_time=30
   ```

`cargo run` builds the harnesses in debug mode and then passes any
arguments after `--` to libFuzzer.  `-max_total_time` limits the run so
these can execute in CI without timing out.  For longer fuzzing sessions
use release mode (`--release`) and a longer time budget.

To simply verify that the harnesses build, the CI pipeline executes each
fuzzer for a single iteration:

```bash
cargo run -p fuzz --bin protocol_frame_decode_fuzz -- -runs=1
cargo run -p fuzz --bin filters_parse_fuzz -- -runs=1
```

## Adding a new corpus

Fuzzers expect input corpora in the directory specified on the command
line.  When no corpus is provided libFuzzer will start from an empty one.
Seed corpora can be supplied by adding paths after the target name:

```bash
cargo run -p fuzz --bin protocol_frame_decode_fuzz corpus_dir -- -max_total_time=60
```
