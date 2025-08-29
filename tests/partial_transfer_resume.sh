#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
RSYNC_RS="$ROOT/target/debug/rsync-rs"

# Ensure binary is built
cargo build --quiet -p rsync-rs-bin --bin rsync-rs

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/rsync_dst" "$TMP/rsync_rs_dst"

printf 'hello world' > "$TMP/src/a.txt"
head -c 5 "$TMP/src/a.txt" > "$TMP/rsync_dst/a.txt"
head -c 5 "$TMP/src/a.txt" > "$TMP/rsync_rs_dst/a.partial"

rsync_output=$(rsync --quiet --partial "$TMP/src/a.txt" "$TMP/rsync_dst/" 2>&1)
rsync_status=$?

rsync_rs_raw=$("$RSYNC_RS" --local --partial "$TMP/src/" "$TMP/rsync_rs_dst/" 2>&1)
rsync_rs_status=$?
rsync_rs_output=$(echo "$rsync_rs_raw" | grep -v -e 'recursive mode enabled' || true)

if [ "$rsync_status" -ne "$rsync_rs_status" ]; then
  echo "Exit codes differ: rsync=$rsync_status rsync-rs=$rsync_rs_status" >&2
  exit 1
fi

if [ "$rsync_output" != "$rsync_rs_output" ]; then
  echo "Outputs differ" >&2
  diff -u <(printf "%s" "$rsync_output") <(printf "%s" "$rsync_rs_output") >&2 || true
  exit 1
fi

if ! diff "$TMP/rsync_dst/a.txt" "$TMP/rsync_rs_dst/a.txt" >/dev/null; then
  echo "Files differ" >&2
  diff "$TMP/rsync_dst/a.txt" "$TMP/rsync_rs_dst/a.txt" >&2 || true
  exit 1
fi
