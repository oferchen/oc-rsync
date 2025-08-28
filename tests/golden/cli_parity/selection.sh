#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
RSYNC_RS="$ROOT/target/debug/rsync-rs"

# temporary directories
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/rsync_dst" "$TMP/rsync_rs_dst"

echo foo > "$TMP/src/a.txt"
echo bar > "$TMP/src/b.log"

rsync_output=$(rsync --quiet --recursive --filter='+ *.txt' --filter='- *' "$TMP/src/" "$TMP/rsync_dst" 2>&1)
rsync_status=$?

rsync_rs_raw=$("$RSYNC_RS" --local --recursive --filter='+ *.txt' --filter='- *' "$TMP/src/" "$TMP/rsync_rs_dst" 2>&1)
rsync_rs_status=$?
rsync_rs_output=$(echo "$rsync_rs_raw" | grep -v 'recursive mode enabled' || true)

if [ "$rsync_status" -ne "$rsync_rs_status" ]; then
  echo "Exit codes differ: rsync=$rsync_status rsync-rs=$rsync_rs_status" >&2
  exit 1
fi

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
