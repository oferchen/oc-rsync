#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
RSYNC_RS="$ROOT/target/debug/rsync-rs"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

run_case() {
  local flags=("$@")
  local dir="$TMP/$(echo "${flags[*]}" | tr -d ' -.')"
  mkdir -p "$dir/src" "$dir/rsync_dst" "$dir/rsync_rs_dst"
  echo keep > "$dir/src/a.txt"
  cp "$dir/src/a.txt" "$dir/rsync_dst/a.txt"
  cp "$dir/src/a.txt" "$dir/rsync_rs_dst/a.txt"

  if [[ " ${flags[*]} " == *" --delete-excluded "* ]]; then
    echo skip > "$dir/src/old.txt"
    cp "$dir/src/old.txt" "$dir/rsync_dst/old.txt"
    cp "$dir/src/old.txt" "$dir/rsync_rs_dst/old.txt"
  else
    echo obsolete > "$dir/rsync_dst/old.txt"
    cp "$dir/rsync_dst/old.txt" "$dir/rsync_rs_dst/old.txt"
  fi

  rsync_output=$(rsync --recursive "${flags[@]}" "$dir/src/" "$dir/rsync_dst" 2>&1)
  rsync_status=$?

  set +e
  rsync_rs_raw=$("$RSYNC_RS" --local --recursive "${flags[@]}" "$dir/src/" "$dir/rsync_rs_dst" 2>&1)
  rsync_rs_status=$?
  set -e
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

  if ! diff -r "$dir/rsync_dst" "$dir/rsync_rs_dst" >/dev/null; then
    echo "Directory trees differ" >&2
    diff -r "$dir/rsync_dst" "$dir/rsync_rs_dst" >&2 || true
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
