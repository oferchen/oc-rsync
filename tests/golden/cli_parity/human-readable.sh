#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/rsync_dst" "$TMP/oc_rsync_dst"

# Create a 2 KiB file
dd if=/dev/zero of="$TMP/src/file" bs=1024 count=2 >/dev/null 2>&1

# Reference copy using rsync
rsync --stats --recursive "$TMP/src/" "$TMP/rsync_dst/" >/dev/null

oc_rsync_raw=$("$OC_RSYNC" --local --stats --progress --human-readable --recursive "$TMP/src/" "$TMP/oc_rsync_dst/" 2>&1)
oc_rsync_status=$?
oc_rsync_progress=$(echo "$oc_rsync_raw" | grep -F "$TMP/oc_rsync_dst/file" || true)
oc_rsync_output=$(echo "$oc_rsync_raw" | grep 'bytes transferred' || true)

if [ "$oc_rsync_status" -ne 0 ]; then
  echo "oc-rsync exited with $oc_rsync_status" >&2
  exit 1
fi

expected_progress="$TMP/oc_rsync_dst/file: 2.00KiB"
if [ "$oc_rsync_progress" != "$expected_progress" ]; then
  echo "Progress outputs differ" >&2
  diff -u <(printf "%s" "$expected_progress") <(printf "%s" "$oc_rsync_progress") >&2 || true
  exit 1
fi

expected="bytes transferred: 2.00KiB"
if [ "$oc_rsync_output" != "$expected" ]; then
  echo "Outputs differ" >&2
  diff -u <(printf "%s" "$expected") <(printf "%s" "$oc_rsync_output") >&2 || true
  exit 1
fi

if ! diff -r "$TMP/rsync_dst" "$TMP/oc_rsync_dst" >/dev/null; then
  echo "Directory trees differ" >&2
  diff -r "$TMP/rsync_dst" "$TMP/oc_rsync_dst" >&2 || true
  exit 1
fi
