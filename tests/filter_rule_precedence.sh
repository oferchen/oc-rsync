#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
RSYNC_RS="$ROOT/target/debug/rsync-rs"

# Ensure binary is built
cargo build --quiet -p rsync-rs-bin --bin rsync-rs

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src/keep/sub" "$TMP/src/keep/tmp" "$TMP/src/skip" "$TMP/rsync_dst" "$TMP/rsync_rs_dst"

# Populate source tree

echo keep > "$TMP/src/keep/file.txt"
echo sub > "$TMP/src/keep/sub/file.md"
echo tmp > "$TMP/src/keep/tmp/file.tmp"
echo skip > "$TMP/src/skip/file.txt"
echo root > "$TMP/src/root.tmp"

# Run reference rsync
rsync_output=$(rsync --quiet --recursive \
  --filter='- *.tmp' \
  --filter='+ keep/tmp/file.tmp' \
  --filter='- skip/' \
  --filter='+ keep/***' \
  --filter='+ *.md' \
  --filter='- *' \
  "$TMP/src/" "$TMP/rsync_dst" 2>&1)
rsync_status=$?

# Run rsync-rs
rsync_rs_raw=$("$RSYNC_RS" --local --recursive \
  --filter='- *.tmp' \
  --filter='+ keep/tmp/file.tmp' \
  --filter='- skip/' \
  --filter='+ keep/***' \
  --filter='+ *.md' \
  --filter='- *' \
  "$TMP/src/" "$TMP/rsync_rs_dst" 2>&1)
rsync_rs_status=$?
rsync_rs_output=$(echo "$rsync_rs_raw" | grep -v -e 'recursive mode enabled' || true)

# Compare exit codes
if [ "$rsync_status" -ne "$rsync_rs_status" ]; then
  echo "Exit codes differ: rsync=$rsync_status rsync-rs=$rsync_rs_status" >&2
  exit 1
fi

# Compare outputs
if [ "$rsync_output" != "$rsync_rs_output" ]; then
  echo "Outputs differ" >&2
  diff -u <(printf "%s" "$rsync_output") <(printf "%s" "$rsync_rs_output") >&2 || true
  exit 1
fi

# Compare directory trees
if ! diff -r "$TMP/rsync_dst" "$TMP/rsync_rs_dst" >/dev/null; then
  echo "Directory trees differ" >&2
  diff -r "$TMP/rsync_dst" "$TMP/rsync_rs_dst" >&2 || true
  exit 1
fi
