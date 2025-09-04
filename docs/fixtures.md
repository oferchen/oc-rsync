# Fixture Generation

The test suite uses fixtures derived from upstream `rsync` to validate CLI parity.

## Version output

`oc-rsync --version` tests compare against the crate's rendered version banner and
no longer require storing upstream `rsync --version` output. Earlier revisions of the
project used a `tests/fixtures/rsync-version.txt` snapshot for this purpose.

## Block-size stats

Block-size unit tests load snapshot `--stats` output captured from upstream
`rsync 3.4.1`. These goldens live under `tests/golden/block_size/` and allow the
tests to run deterministically without invoking a system `rsync` binary.
