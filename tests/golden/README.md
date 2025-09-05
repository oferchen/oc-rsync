# Golden Outputs

This directory stores pre-recorded stdout and exit codes from upstream
`rsync` (version 3.4.1). Tests consume these files instead of executing
`rsync` directly.

## Regenerating

1. Ensure `rsync` 3.4.1 is installed and available on `PATH`.
2. Recreate the relevant scenarios and capture their results. For example:

```bash
RSYNC_CHECKSUM_LIST=sha1 rsync --quiet --recursive --checksum src/ dst/ \
  > tests/golden/cli_parity/checksum-choice.rsync1.stdout 2>&1
printf '%s' $? > tests/golden/cli_parity/checksum-choice.rsync1.status
```

3. Commit the updated files under `tests/golden/**`.
