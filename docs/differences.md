# Behavioral Differences

This document enumerates observable divergences between `oc-rsync` and classic
`rsync`. It should become empty once full parity is achieved.


- `--numeric-ids` currently requires root or `CAP_CHOWN` and may not resolve
  IDs exactly as upstream does.
