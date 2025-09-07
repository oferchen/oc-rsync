# Interoperability Grid

`scripts/interop-grid.sh` compares `oc-rsync` with the stock `rsync` binary across a grid of common options. Every combination of `--archive`, `--compress`, and `--delete` is executed against a small local tree.

For each flag set the script runs both implementations, capturing stdout, stderr, and exit codes. After each transfer the source tree is compared against the destination using `rsync -aiXn` to verify metadata including permissions, timestamps, and xattrs. Any output or non‑zero exit status causes the script to fail, and differences are noted in the report.

```
scripts/interop-grid.sh
```

Results are written to `tests/interop/interop-grid.log` for inspection alongside other interoperability fixtures.

`scripts/interop/run.sh` provides a lower‑level view of these transfers.  It
records each `rsync` invocation's stdout, stderr, and exit code in versioned
files under `tests/interop/streams/`.  The companion
`scripts/interop/validate.sh` compares the captured streams from `oc-rsync` and
upstream `rsync`, failing fast on any mismatch in output or exit status.

## Extended matrix coverage

`scripts/interop.sh` exercises a wider set of options to ensure
parity with upstream `rsync`. Recent runs include the following flags and all
combinations completed without mismatches:

| Flag | Result |
| --- | --- |
| `--partial` | ✅ |
| `--inplace` | ✅ |
| `--progress` | ✅ |
| `--info=progress2` | ✅ |
