# Fixture Generation

The fixtures in this directory capture upstream and oc-rsync help output in a
locale-independent way.


## `rsync-help-80.txt` and `rsync-help-120.txt`

These files capture the full `oc-rsync --help` output at specific widths.
`rsync-help-80.txt` now lives under `crates/cli/resources/` for inclusion in the
CLI crate. Regenerate them in the C locale with:

```sh
LC_ALL=C LANG=C COLUMNS=80  cargo run --bin oc-rsync -- --help > crates/cli/resources/rsync-help-80.txt
LC_ALL=C LANG=C COLUMNS=120 cargo run --bin oc-rsync -- --help > tests/fixtures/rsync-help-120.txt
```

## `oc-rsync-help.txt` and `oc-rsync-version.txt`

These fixtures capture the current `oc-rsync` command's `--help` and `--version`
outputs. Regenerate them with:

```sh
LC_ALL=C LANG=C COLUMNS=80 cargo run --bin oc-rsync -- --help > tests/fixtures/oc-rsync-help.txt
LC_ALL=C LANG=C cargo run --bin oc-rsync -- --version > tests/fixtures/oc-rsync-version.txt
```

## Upstream interaction goldens

Some tests compare `oc-rsync`'s behavior against fixtures generated from
upstream `rsync` output.  To refresh those fixtures, run:

```
make refresh-upstream-goldens
```

This target uses a pinned upstream container to regenerate files under
`tests/fixtures/`.  Tests will fail if these fixtures drift from the committed
versions.
