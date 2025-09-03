# Fixture Generation

The help text fixtures are used to ensure `oc-rsync --help` matches the
expected output.

To regenerate `rsync-help.txt`, run the binary with the C locale to avoid
locale-specific differences:

```sh
LC_ALL=C LANG=C COLUMNS=80 cargo run --bin oc-rsync -- --help > tests/fixtures/rsync-help.txt
```

For wider help output, replace `COLUMNS=80` with the desired width and write
that output to a separate fixture file.
