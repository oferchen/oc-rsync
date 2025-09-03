# Fixture Generation

The test suite uses fixtures derived from upstream `rsync` to validate CLI parity.

## `rsync-version.txt`

`tests/fixtures/rsync-version.txt` stores the output of `rsync --version` for comparison
with `oc-rsync --version`.

Regenerate the fixture with:

```sh
LC_ALL=C COLUMNS=80 rsync --version > tests/fixtures/rsync-version.txt
```

Setting `LC_ALL` and `COLUMNS` ensures a stable locale and line wrapping.
