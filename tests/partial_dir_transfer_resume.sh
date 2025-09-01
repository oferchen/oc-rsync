#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

# Ensure binary is built
cargo build --quiet -p oc-rsync-bin --bin oc-rsync --features blake3

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/rsync_dst/partial" "$TMP/oc_rsync_dst/partial"

printf 'hello world' > "$TMP/src/a.txt"
head -c 5 "$TMP/src/a.txt" > "$TMP/rsync_dst/partial/a.txt"
head -c 5 "$TMP/src/a.txt" > "$TMP/oc_rsync_dst/partial/a.txt"

rsync_output=$(rsync --quiet --partial-dir=partial "$TMP/src/a.txt" "$TMP/rsync_dst/" 2>&1)
rsync_status=$?

oc_rsync_raw=$("$OC_RSYNC" --local --partial --partial-dir partial "$TMP/src/" "$TMP/oc_rsync_dst/" 2>&1)
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

if ! diff "$TMP/rsync_dst/a.txt" "$TMP/oc_rsync_dst/a.txt" >/dev/null; then
  echo "Files differ" >&2
  diff "$TMP/rsync_dst/a.txt" "$TMP/oc_rsync_dst/a.txt" >&2 || true
  exit 1
fi
