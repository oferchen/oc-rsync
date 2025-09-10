# Rolling checksum

This module implements the 32-bit rolling checksum used by rsync. It
maintains two 16-bit accumulators, `s1` and `s2`, which are combined into
a single 32-bit value.

## Invariants

- `s1` is the sum of all bytes in the window, wrapping on overflow.
- `s2` is the sum of the `s1` values as the window advances.
- `Rolling::len` is fixed after construction and describes the window
  size used by [`Rolling::roll`].
- `Rolling::digest` combines `s1` and `s2` as `(s1 & 0xffff) | (s2 << 16)`.

## Safety

The SIMD optimised implementations are `unsafe` because they rely on CPU
features (SSE4.2, AVX2, AVX-512) that the compiler cannot guarantee are
present at runtime. Calling them on unsupported hardware results in
undefined behaviour. The safe entry points—`rolling_checksum`,
`rolling_checksum_seeded`, and the [`RollingChecksum`] trait
implementations—perform feature detection before dispatching to these
routines and should be preferred by callers.
