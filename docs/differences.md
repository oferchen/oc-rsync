# Behavioral Differences

This document enumerates observable divergences between `oc-rsync` and classic
`rsync`. It should become empty once full parity is achieved.

- `--archive` does not yet enable the complete set of flags implied by upstream
  `rsync -a`.
- `--log-file` and `--log-file-format` accept only a subset of format escape
  sequences.
- `--dry-run` output and exit codes may differ when deletions or errors occur.
- `--progress` and `--stats` output formatting differs from upstream.
- `--numeric-ids` currently requires root or `CAP_CHOWN` and may not resolve
  IDs exactly as upstream does.

