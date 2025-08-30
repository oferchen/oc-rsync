#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
RSYNC_RS="$ROOT/target/debug/rsync-rs"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/rsync_dst" "$TMP/rsync_rs_dst"

# Create a 2 KiB file
dd if=/dev/zero of="$TMP/src/file" bs=1024 count=2 >/dev/null 2>&1

# Reference copy using rsync
rsync --stats --recursive "$TMP/src/" "$TMP/rsync_dst/" >/dev/null

rsync_rs_raw=$("$RSYNC_RS" --local --stats --human-readable --recursive "$TMP/src/" "$TMP/rsync_rs_dst/" 2>&1)
rsync_rs_status=$?
rsync_rs_output=$(echo "$rsync_rs_raw" | grep 'bytes transferred' || true)

if [ "$rsync_rs_status" -ne 0 ]; then
  echo "rsync-rs exited with $rsync_rs_status" >&2
  exit 1
fi

expected="bytes transferred: 2.00KiB"
if [ "$rsync_rs_output" != "$expected" ]; then
  echo "Outputs differ" >&2
  diff -u <(printf "%s" "$expected") <(printf "%s" "$rsync_rs_output") >&2 || true
  exit 1
fi

if ! diff -r "$TMP/rsync_dst" "$TMP/rsync_rs_dst" >/dev/null; then
  echo "Directory trees differ" >&2
  diff -r "$TMP/rsync_dst" "$TMP/rsync_rs_dst" >&2 || true
  exit 1
fi
