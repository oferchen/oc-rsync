This directory stores recorded wire transcripts, file-list goldens and
filesystem trees for interoperability tests.

- `wire/` contains captured protocol transcripts. Legacy transcripts may
  remain for historical coverage; tests using them automatically skip when
  a protocol version isn't supported.
- `filelists/` stores `rsync --list-only` outputs.
- `golden/` holds destination tree tarballs for interoperability tests. Each
  tarball is stored at
  `golden/<client>_<server>_<transport>/tree.tar` where `client` and `server`
  may be `oc-rsync` or `upstream`.
- `run_matrix.sh` runs a matrix of rsync client/server combinations over both SSH
  and rsync:// transports. Set `UPDATE=1` to regenerate tarball goldens.

The committed goldens were recorded using `run_matrix.sh` against upstream
`rsync` and the local `oc-rsync` build. To record fresh goldens, point
`UPSTREAM_RSYNC` at a built `rsync` binary and limit the scenarios to `base`:

```
SCENARIOS=base UPDATE=1 \
  CLIENT_VERSIONS="upstream oc-rsync" \
  SERVER_VERSIONS="upstream oc-rsync" \
  UPSTREAM_RSYNC=/path/to/rsync tests/interop/run_matrix.sh
```
- `interop-grid.log` is produced by `scripts/interop-grid.sh` and captures exit
  codes and stderr comparisons for key flag combinations.

The CI workflow executes `run_matrix.sh` as a required status check to ensure
interoperability across supported rsync versions.
