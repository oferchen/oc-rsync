This directory stores recorded wire transcripts, file-list goldens and
filesystem trees for interoperability tests.

- `wire/` contains captured protocol transcripts.
- `filelists/` stores `rsync --list-only` outputs.
- `golden/` holds destination trees for rsync 3.2.x client/server
  interoperability tests. Trees are organized by
  `<client>_<server>_<transport>`.
- `run_matrix.sh` runs a matrix of rsync 3.2.x client/server combinations over
  both SSH and rsync:// transports. Set `UPDATE=1` to regenerate goldens.
- `interop-grid.log` is produced by `scripts/interop-grid.sh` and captures exit
  codes and stderr comparisons for key flag combinations.

The CI workflow executes `run_matrix.sh` as a required status check to ensure
interoperability across supported rsync versions.
