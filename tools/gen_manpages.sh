#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT="$ROOT/docs/man"
BIN="${1:-$ROOT/target/debug/oc-rsync}"

export LC_ALL=C
export COLUMNS=80

mkdir -p "$OUT"
help2man --no-info "$BIN" > "$OUT/oc-rsync.1"
scdoc < "$ROOT/docs/oc-rsyncd.conf.scd" > "$OUT/oc-rsyncd.conf.5"
