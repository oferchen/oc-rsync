#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"
GOLDEN="$ROOT/tests/golden/cli_parity"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/oc_rsync_dst"

echo data > "$TMP/src/a.txt"

rsync_output=$(cat "$GOLDEN/checksum-choice.rsync1.stdout")
rsync_status=$(cat "$GOLDEN/checksum-choice.rsync1.status")
RSYNC_CHECKSUM_LIST=sha1 oc_rsync_raw=$("$OC_RSYNC" --recursive --checksum "$TMP/src/" "$TMP/oc_rsync_dst" 2>&1)
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

if ! diff -r "$TMP/src" "$TMP/oc_rsync_dst" >/dev/null; then
  echo "Directory trees differ" >&2
  diff -r "$TMP/src" "$TMP/oc_rsync_dst" >&2 || true
  exit 1
fi

rm -rf "$TMP/oc_rsync_dst"
mkdir -p "$TMP/oc_rsync_dst"

rsync_output=$(cat "$GOLDEN/checksum-choice.rsync2.stdout")
rsync_status=$(cat "$GOLDEN/checksum-choice.rsync2.status")

oc_rsync_raw=$("$OC_RSYNC" --recursive --checksum --cc=sha1 "$TMP/src/" "$TMP/oc_rsync_dst" 2>&1)
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

if ! diff -r "$TMP/src" "$TMP/oc_rsync_dst" >/dev/null; then
  echo "Directory trees differ" >&2
  diff -r "$TMP/src" "$TMP/oc_rsync_dst" >&2 || true
  exit 1
fi
