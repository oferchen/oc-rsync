#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

# Ensure binary is built
cargo build --quiet --bin oc-rsync --features blake3

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src/keep/sub" "$TMP/src/keep/tmp" "$TMP/src/skip" "$TMP/rsync_dst" "$TMP/oc_rsync_dst"

# Populate source tree

echo keep > "$TMP/src/keep/file.txt"
echo sub > "$TMP/src/keep/sub/file.md"
echo tmp > "$TMP/src/keep/tmp/file.tmp"
echo skip > "$TMP/src/skip/file.txt"
echo root > "$TMP/src/root.tmp"
echo core > "$TMP/src/core"
echo obj > "$TMP/src/foo.o"
echo debug > "$TMP/src/debug.log"
echo info > "$TMP/src/info.log"

# Run reference rsync
rsync_output=$(rsync --quiet --recursive \
  --filter='+ core' \
  --filter='-C' \
  --filter='- *.tmp' \
  --filter='S debug.log' \
  --filter='- *.log' \
  --filter='+ keep/tmp/file.tmp' \
  --filter='- skip/' \
  --filter='+ keep/***' \
  --filter='+ *.md' \
  --filter='- *' \
  "$TMP/src/" "$TMP/rsync_dst" 2>&1)
rsync_status=$?

# Run oc-rsync
oc_rsync_raw=$("$OC_RSYNC" --local --recursive \
  --filter='+ core' \
  --filter='-C' \
  --filter='- *.tmp' \
  --filter='S debug.log' \
  --filter='- *.log' \
  --filter='+ keep/tmp/file.tmp' \
  --filter='- skip/' \
  --filter='+ keep/***' \
  --filter='+ *.md' \
  --filter='- *' \
  "$TMP/src/" "$TMP/oc_rsync_dst" 2>&1)
oc_rsync_status=$?
oc_rsync_output=$(echo "$oc_rsync_raw" | grep -v -e 'recursive mode enabled' || true)

# Compare exit codes
if [ "$rsync_status" -ne "$oc_rsync_status" ]; then
  echo "Exit codes differ: rsync=$rsync_status oc-rsync=$oc_rsync_status" >&2
  exit 1
fi

# Compare outputs
if [ "$rsync_output" != "$oc_rsync_output" ]; then
  echo "Outputs differ" >&2
  diff -u <(printf "%s" "$rsync_output") <(printf "%s" "$oc_rsync_output") >&2 || true
  exit 1
fi

# Compare directory trees
if ! diff -r "$TMP/rsync_dst" "$TMP/oc_rsync_dst" >/dev/null; then
  echo "Directory trees differ" >&2
  diff -r "$TMP/rsync_dst" "$TMP/oc_rsync_dst" >&2 || true
  exit 1
fi
