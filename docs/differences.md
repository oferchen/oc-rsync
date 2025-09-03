# Differences from upstream rsync

- Access times (`--atimes`) are not preserved by default. Explicitly enable them with the `--atimes` flag or via `SyncConfig::atimes(true)` until full parity with upstream rsync is verified.

This document tracks behavioral or feature differences between oc-rsync and upstream rsync. Update this file whenever a divergence is discovered or resolved.
