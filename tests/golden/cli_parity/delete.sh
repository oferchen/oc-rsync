#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

run_case() {
  local flags=("$@")
  local dir="$TMP/$(echo "${flags[*]}" | tr -d ' -.')"
  mkdir -p "$dir/src" "$dir/rsync_dst" "$dir/oc_rsync_dst"
  echo keep > "$dir/src/a.txt"
  cp "$dir/src/a.txt" "$dir/rsync_dst/a.txt"
  cp "$dir/src/a.txt" "$dir/oc_rsync_dst/a.txt"

  if [[ " ${flags[*]} " == *" --delete-excluded "* ]]; then
    echo skip > "$dir/src/old.txt"
    cp "$dir/src/old.txt" "$dir/rsync_dst/old.txt"
    cp "$dir/src/old.txt" "$dir/oc_rsync_dst/old.txt"
  else
    echo obsolete > "$dir/rsync_dst/old.txt"
    cp "$dir/rsync_dst/old.txt" "$dir/oc_rsync_dst/old.txt"
  fi

  rsync_output=$(rsync --recursive "${flags[@]}" "$dir/src/" "$dir/rsync_dst" 2>&1)
  rsync_status=$?

  set +e
  oc_rsync_raw=$("$OC_RSYNC" --recursive "${flags[@]}" "$dir/src/" "$dir/oc_rsync_dst" 2>&1)
  oc_rsync_status=$?
  set -e
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

  if ! diff -r "$dir/rsync_dst" "$dir/oc_rsync_dst" >/dev/null; then
    echo "Directory trees differ" >&2
    diff -r "$dir/rsync_dst" "$dir/oc_rsync_dst" >&2 || true
    exit 1
  fi
}

run_case --delete
run_case --delete-before
run_case --delete-after
run_case --delete-delay
run_case --delete-during
run_case --del
run_case --delete-excluded --exclude old.txt
