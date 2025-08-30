# Performance

This document highlights the impact of new runtime CPU feature detection for
compression and rolling checksums. When AVX2/AVX-512/SSE4.2 are available the
engine selects optimized code paths while maintaining scalar fallbacks for other
architectures.

Benchmarks are available under `crates/engine/benches`. Running on the default
CI environment produced the following sample results:

```
$ cargo bench -p engine --bench rolling -- --sample-size=10
rolling_checksum_1mb   time:   [108.09 µs 108.57 µs 109.27 µs]
$ cargo bench -p engine --bench compress -- --sample-size=10
zstd_compress_1mb      time:   [378.50 µs 382.96 µs 387.35 µs]
```

Actual numbers will vary by hardware, but AVX2 consistently improves rolling
checksum throughput compared to the scalar version while leaving behavior
unchanged on CPUs without these extensions.

## Coverage

This project uses [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) to aggregate
unit tests, integration tests, and documentation examples into a unified coverage report.

Run locally with:

```
cargo llvm-cov --all-features --workspace --doctests \
  --fail-under-lines 95 --fail-under-functions 95
```

The command above enforces a 95% threshold for both line and function coverage,
matching the CI gate.
