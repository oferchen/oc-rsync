#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

# temporary directories
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/rsync_dst" "$TMP/oc_rsync_dst"

echo foo > "$TMP/src/a.txt"
echo bar > "$TMP/src/b.log"

rsync_output=$(rsync --quiet --recursive --filter='+ *.txt' --filter='- *' "$TMP/src/" "$TMP/rsync_dst" 2>&1)
rsync_status=$?

oc_rsync_raw=$("$OC_RSYNC" --local --recursive --filter='+ *.txt' --filter='- *' "$TMP/src/" "$TMP/oc_rsync_dst" 2>&1)
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

# --dirs copies directory structure only
rm -rf "$TMP/src" "$TMP/rsync_dst" "$TMP/oc_rsync_dst"
mkdir -p "$TMP/src/sub" "$TMP/rsync_dst" "$TMP/oc_rsync_dst"
echo data > "$TMP/src/sub/file.txt"

rsync_output=$(rsync --quiet --dirs "$TMP/src/" "$TMP/rsync_dst" 2>&1)
rsync_status=$?

oc_rsync_raw=$("$OC_RSYNC" --local --dirs "$TMP/src/" "$TMP/oc_rsync_dst" 2>&1)
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

# --list-only lists files without copying
rm -rf "$TMP/src" "$TMP/rsync_dst" "$TMP/oc_rsync_dst"
mkdir -p "$TMP/src" "$TMP/rsync_dst" "$TMP/oc_rsync_dst"
echo foo > "$TMP/src/a.txt"
echo bar > "$TMP/src/b.txt"

rsync_output=$(rsync --list-only "$TMP/src/" "$TMP/rsync_dst" 2>&1 | awk '{print $NF}')
rsync_status=$?

oc_rsync_output=$("$OC_RSYNC" --local --list-only "$TMP/src/" "$TMP/oc_rsync_dst" 2>&1 | awk '{print $NF}')
oc_rsync_status=$?

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

# --update skips newer destination files
rm -rf "$TMP/src" "$TMP/rsync_dst" "$TMP/oc_rsync_dst"
mkdir -p "$TMP/src" "$TMP/rsync_dst" "$TMP/oc_rsync_dst"
echo new > "$TMP/src/file.txt"
echo old > "$TMP/rsync_dst/file.txt"
cp "$TMP/rsync_dst/file.txt" "$TMP/oc_rsync_dst/file.txt"
touch -d '2038-01-01' "$TMP/rsync_dst/file.txt" "$TMP/oc_rsync_dst/file.txt"

rsync_output=$(rsync --quiet --update "$TMP/src/" "$TMP/rsync_dst" 2>&1)
rsync_status=$?

oc_rsync_output=$("$OC_RSYNC" --local --update "$TMP/src/" "$TMP/oc_rsync_dst" 2>&1)
oc_rsync_status=$?

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
