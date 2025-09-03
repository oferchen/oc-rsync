# Fixture Generation

The test suite uses fixtures derived from upstream `rsync` to validate CLI parity.

## Version output

`oc-rsync --version` tests compare against the crate's rendered version banner and
no longer require storing upstream `rsync --version` output. Earlier revisions of the
project used a `tests/fixtures/rsync-version.txt` snapshot for this purpose.
