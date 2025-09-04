#!/bin/bash
set -euo pipefail
export LC_ALL=C LANG=C COLUMNS=80 TZ=UTC
FIXTURES="$(git rev-parse --show-toplevel)/tests/fixtures"

# dry_run deletions
work="$(mktemp -d)"
mkdir "$work/src" "$work/dst"
echo old > "$work/dst/old.txt"
rsync -av --delete --dry-run "$work/src/" "$work/dst" | grep '^deleting ' > "$FIXTURES/dry_run_deletions.txt"
rm -r "$work"

# dry_run errors
work="$(mktemp -d)"
mkdir "$work/dst"
set +e
err=$(rsync --dry-run missing.txt "$work/dst" 2>&1)
set -e
err=${err//"$work"\/missing.txt/{path}}
printf "%s\n" "$err" > "$FIXTURES/dry_run_errors.stderr"
rm -r "$work"

# remote_remote via rsh hash
work="$(mktemp -d)"
mkdir "$work/src"
echo via_rsh > "$work/src/file.txt"
chmod 600 "$work/src/file.txt"
ln -s file.txt "$work/src/link.txt"
tar --numeric-owner -cf - -C "$work/src" . | sha256sum | cut -d' ' -f1 > "$FIXTURES/remote_remote_via_rsh.hash"
rm -r "$work"

# remote_remote via rsync urls hash
work="$(mktemp -d)"
mkdir "$work/src"
echo via_daemon > "$work/src/file.txt"
chmod 640 "$work/src/file.txt"
ln -s file.txt "$work/src/link.txt"
tar --numeric-owner -cf - -C "$work/src" . | sha256sum | cut -d' ' -f1 > "$FIXTURES/remote_remote_via_rsync_urls.hash"
rm -r "$work"
