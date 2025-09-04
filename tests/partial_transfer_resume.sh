#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

# Ensure binary is built
cargo build --quiet -p oc-rsync-bin --bin oc-rsync

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/oc_rsync_dst"

printf 'hello world' > "$TMP/src/a.txt"
head -c 5 "$TMP/src/a.txt" > "$TMP/oc_rsync_dst/a.partial"

expected_output=$(cat "$ROOT/tests/golden/partial_transfer_resume/output.txt")
expected_status=$(cat "$ROOT/tests/golden/partial_transfer_resume/exit_code")

oc_rsync_raw=$("$OC_RSYNC" --partial "$TMP/src/" "$TMP/oc_rsync_dst/" 2>&1)
oc_rsync_status=$?
oc_rsync_output=$(echo "$oc_rsync_raw" | grep -v -e 'recursive mode enabled' || true)

if [ "$expected_status" -ne "$oc_rsync_status" ]; then
  echo "Exit codes differ: expected=$expected_status oc-rsync=$oc_rsync_status" >&2
  exit 1
fi

if [ "$expected_output" != "$oc_rsync_output" ]; then
  echo "Outputs differ" >&2
  diff -u <(printf "%s" "$expected_output") <(printf "%s" "$oc_rsync_output") >&2 || true
  exit 1
fi

if ! diff "$ROOT/tests/golden/partial_transfer_resume/a.txt" "$TMP/oc_rsync_dst/a.txt" >/dev/null; then
  echo "Files differ" >&2
  diff "$ROOT/tests/golden/partial_transfer_resume/a.txt" "$TMP/oc_rsync_dst/a.txt" >&2 || true
  exit 1
fi
