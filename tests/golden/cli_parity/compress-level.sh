#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/rsync_dst" "$TMP/oc_rsync_dst"

echo data > "$TMP/src/a.txt"

rsync_output=$(rsync --quiet --recursive --compress-level=1 "$TMP/src/" "$TMP/rsync_dst" 2>&1)
rsync_status=$?

oc_rsync_raw=$("$OC_RSYNC" --recursive --compress-level=1 "$TMP/src/" "$TMP/oc_rsync_dst" 2>&1)
oc_rsync_status=$?
oc_rsync_output=$(echo "$oc_rsync_raw" | grep -v 'recursive mode enabled' || true)

if [ "$rsync_status" -ne "$oc_rsync_status" ]; then
  echo "Exit codes differ: rsync=$rsync_status oc-rsync=$oc_rsync_status" >&2
  exit 1
fi

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

rm -rf "$TMP/rsync_dst" "$TMP/oc_rsync_dst"
mkdir -p "$TMP/rsync_dst" "$TMP/oc_rsync_dst"

rsync_output=$(rsync --quiet --recursive --zl=1 "$TMP/src/" "$TMP/rsync_dst" 2>&1)
rsync_status=$?

oc_rsync_raw=$("$OC_RSYNC" --recursive --zl=1 "$TMP/src/" "$TMP/oc_rsync_dst" 2>&1)
oc_rsync_status=$?
oc_rsync_output=$(echo "$oc_rsync_raw" | grep -v 'recursive mode enabled' || true)

if [ "$rsync_status" -ne "$oc_rsync_status" ]; then
  echo "Exit codes differ: rsync=$rsync_status oc-rsync=$oc_rsync_status" >&2
  exit 1
fi

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
