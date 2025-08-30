#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src/subdir" "$TMP/rsync_dst" "$TMP/oc_rsync_dst"

echo data > "$TMP/src/a.txt"
echo data > "$TMP/src/subdir/b.txt"
ln -s a.txt "$TMP/src/link"

rsync_output=$(rsync --recursive --links --itemize-changes "$TMP/src/" "$TMP/rsync_dst" 2>&1 | grep -v 'sending incremental file list')

oc_rsync_raw=$("$OC_RSYNC" --local --recursive --links --itemize-changes "$TMP/src/" "$TMP/oc_rsync_dst" 2>&1)
oc_rsync_output=$(echo "$oc_rsync_raw" | grep -v -e 'recursive mode enabled' || true)

if [ "$rsync_output" != "$oc_rsync_output" ]; then
  echo "Outputs differ" >&2
  diff -u <(printf "%s" "$rsync_output") <(printf "%s" "$oc_rsync_output") >&2 || true
  exit 1
fi

if ! diff -r "$TMP/rsync_dst" "$TMP/oc_rsync_dst" >/dev/null; then
  echo "Directory trees differ" >&2
  diff -r "$TMP/rsync_dst" "$TMP/oc_rsync_dst" >&2 || true
  exit 1
fi
