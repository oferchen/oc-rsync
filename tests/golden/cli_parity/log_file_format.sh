#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$TMP/dst"

echo data > "$TMP/src/a.txt"

log="$TMP/log.json"
"$OC_RSYNC" --log-file "$log" --log-file-format=json -v "$TMP/src/" "$TMP/dst/" >/dev/null

if ! grep -q '"message"' "$log"; then
  echo "log file missing json message" >&2
  cat "$log" >&2
  exit 1
fi
