# Interoperability Grid

`scripts/interop-grid.sh` compares `oc-rsync` with the stock `rsync` binary across a grid of common options. Every combination of `--archive`, `--compress`, and `--delete` is executed against a small local tree.

For each flag set the script runs both implementations, recording exit codes and any stderr output. Differences are noted in the report.

```
scripts/interop-grid.sh
```

Results are written to `tests/interop/interop-grid.log` for inspection alongside other interoperability fixtures.
