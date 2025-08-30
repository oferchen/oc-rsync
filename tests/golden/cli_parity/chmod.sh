#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/rsync_dst" "$TMP/oc_rsync_dst"

echo data > "$TMP/src/a.txt"

rsync_output=$(rsync --quiet --recursive --perms --chmod=go-rwx "$TMP/src/" "$TMP/rsync_dst" 2>&1)
rsync_status=$?

oc_rsync_raw=$("$OC_RSYNC" --local --recursive --perms --chmod=go-rwx "$TMP/src/" "$TMP/oc_rsync_dst" 2>&1)
oc_rsync_status=$?
oc_rsync_output=$(echo "$oc_rsync_raw" | grep -v -e 'recursive mode enabled' || true)

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

rsync_mode=$(/usr/bin/python3 -c 'import os,sys; print(oct(os.stat(sys.argv[1]).st_mode & 0o777))' "$TMP/rsync_dst/a.txt")
oc_rsync_mode=$(/usr/bin/python3 -c 'import os,sys; print(oct(os.stat(sys.argv[1]).st_mode & 0o777))' "$TMP/oc_rsync_dst/a.txt")
if [ "$rsync_mode" != "$oc_rsync_mode" ]; then
  echo "File modes differ: rsync=$rsync_mode oc-rsync=$oc_rsync_mode" >&2
  exit 1
fi
