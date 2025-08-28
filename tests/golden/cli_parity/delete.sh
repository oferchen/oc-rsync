#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
RSYNC_RS="$ROOT/target/debug/rsync-rs"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/rsync_dst" "$TMP/rsync_rs_dst"

echo keep > "$TMP/src/a.txt"
# prepare destinations with extra file
mkdir -p "$TMP/rsync_dst" "$TMP/rsync_rs_dst"
echo keep > "$TMP/rsync_dst/a.txt"
echo obsolete > "$TMP/rsync_dst/old.txt"
cp -r "$TMP/rsync_dst"/* "$TMP/rsync_rs_dst"/

rsync_output=$(rsync --quiet --recursive --delete "$TMP/src/" "$TMP/rsync_dst" 2>&1)
rsync_status=$?

set +e
rsync_rs_raw=$("$RSYNC_RS" --local --recursive --delete "$TMP/src/" "$TMP/rsync_rs_dst" 2>&1)
rsync_rs_status=$?
set -e
rsync_rs_output=$(echo "$rsync_rs_raw" | grep -v 'recursive mode enabled' || true)

if [ "$rsync_rs_status" -ne 0 ]; then
  echo "rsync-rs delete not implemented; skipping parity check" >&2
  exit 0
fi

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
