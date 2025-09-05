#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

# Ensure binary is built
cargo build --quiet -p oc-rsync --bin oc-rsync

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/oc_rsync_dst/partial"

printf 'hello world' > "$TMP/src/a.txt"
head -c 5 "$TMP/src/a.txt" > "$TMP/oc_rsync_dst/partial/a.txt"

expected_output=$(cat "$ROOT/tests/golden/partial_dir_transfer_resume/output.txt")
expected_status=$(cat "$ROOT/tests/golden/partial_dir_transfer_resume/exit_code")

oc_rsync_raw=$("$OC_RSYNC" --partial --partial-dir partial "$TMP/src/" "$TMP/oc_rsync_dst/" 2>&1)
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

if ! diff "$ROOT/tests/golden/partial_dir_transfer_resume/a.txt" "$TMP/oc_rsync_dst/a.txt" >/dev/null; then
  echo "Files differ" >&2
  diff "$ROOT/tests/golden/partial_dir_transfer_resume/a.txt" "$TMP/oc_rsync_dst/a.txt" >&2 || true
  exit 1
fi

# Test daemon mode with nested directories
mkdir -p "$TMP/src/sub" "$TMP/daemon_dst/partial/sub"
printf 'daemon hello' > "$TMP/src/sub/a.txt"
head -c 6 "$TMP/src/sub/a.txt" > "$TMP/daemon_dst/partial/sub/a.txt"
daemon_log="$TMP/daemon.log"
"$OC_RSYNC" --daemon --no-detach --module "data=$TMP/daemon_dst" --port 0 >"$daemon_log" 2>/dev/null &
daemon_pid=$!
for i in {1..50}; do
  if [ -s "$daemon_log" ]; then
    DAEMON_PORT=$(cat "$daemon_log")
    break
  fi
  sleep 0.1
done
if [ -z "${DAEMON_PORT:-}" ]; then
  echo "failed to get daemon port" >&2
  kill "$daemon_pid"
  exit 1
fi
"$OC_RSYNC" --partial --partial-dir partial "$TMP/src/" "rsync://127.0.0.1:$DAEMON_PORT/data/" >/dev/null 2>&1
kill "$daemon_pid"
wait "$daemon_pid" 2>/dev/null || true
if ! diff "$TMP/src/sub/a.txt" "$TMP/daemon_dst/sub/a.txt" >/dev/null; then
  echo "Daemon files differ" >&2
  diff "$TMP/src/sub/a.txt" "$TMP/daemon_dst/sub/a.txt" >&2 || true
  exit 1
fi
