#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
RSYNC_RS="$ROOT/target/debug/rsync-rs"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src/subdir" "$TMP/rsync_dst" "$TMP/rsync_rs_dst"

echo data > "$TMP/src/a.txt"
echo data > "$TMP/src/subdir/b.txt"

rsync_output=$(rsync --recursive --itemize-changes "$TMP/src/" "$TMP/rsync_dst" 2>&1 | grep -v 'sending incremental file list')

rsync_rs_raw=$("$RSYNC_RS" --local --recursive --itemize-changes "$TMP/src/" "$TMP/rsync_rs_dst" 2>&1)
rsync_rs_output=$(echo "$rsync_rs_raw" | grep -v -e 'recursive mode enabled' || true)

if [ "$rsync_output" != "$rsync_rs_output" ]; then
  echo "Outputs differ" >&2
  diff -u <(printf "%s" "$rsync_output") <(printf "%s" "$rsync_rs_output") >&2 || true
  exit 1
fi

if ! diff -r "$TMP/rsync_dst" "$TMP/rsync_rs_dst" >/dev/null; then
  echo "Directory trees differ" >&2
  diff -r "$TMP/rsync_dst" "$TMP/rsync_rs_dst" >&2 || true
  exit 1
fi
