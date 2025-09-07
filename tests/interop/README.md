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
- `scripts/interop.sh` replays a matrix of rsync client/server combinations over both
  SSH and rsync:// transports using the pre-generated fixtures in `golden/`.
  Set `UPDATE=1` to regenerate tarball goldens from a locally built upstream
  `rsync`. Daemon ports are assigned deterministically starting from
  `INTEROP_PORT_BASE` (default `43000`), incrementing for each daemon to ensure
  unique, stable ports across runs.

The committed goldens were recorded in a controlled environment using upstream
`rsync 3.4.1` and the local `oc-rsync` build. To record fresh goldens, point
`UPSTREAM_RSYNC` at a built `rsync` binary and limit the scenarios to `base`:

```
SCENARIOS=base UPDATE=1 \
  CLIENT_VERSIONS="upstream oc-rsync" \
  SERVER_VERSIONS="upstream oc-rsync" \
  UPSTREAM_RSYNC=/path/to/rsync scripts/interop.sh
```
- `interop-grid.log` is produced by `scripts/interop-grid.sh` and captures exit
  codes and stderr comparisons for key flag combinations.

The CI workflow runs `scripts/interop.sh` without network access and relies solely on
the committed fixtures to verify interoperability.

When adding new interoperability scenarios or fixtures, also update the
["Interop matrix scenarios"](../../docs/gaps.md#interop-matrix-scenarios)
section so documentation stays in sync.
