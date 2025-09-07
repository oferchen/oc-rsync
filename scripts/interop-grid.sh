#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"
OUT_DIR="$ROOT/tests/interop"
OUT_FILE="$OUT_DIR/interop-grid.log"

# Build both implementations, run transfers, and compare results.  For each
# flag combination the script captures stdout, stderr, and exit codes for both
# `rsync` and `oc-rsync`.  Source and destination trees are then compared via
# `rsync -aiXn` to verify permissions, timestamps, and xattrs.  Any deviation or
# non‑zero exit code causes the script to terminate with failure.

# Build oc-rsync if needed
if [ ! -x "$OC_RSYNC" ]; then
  echo "Building oc-rsync" >&2
  cargo build --quiet -p oc-rsync --bin oc-rsync
fi

FLAGS=(--archive --compress --delete)
N=${#FLAGS[@]}

mkdir -p "$OUT_DIR"
: > "$OUT_FILE"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

SRC="$TMP/src"
mkdir -p "$SRC"
echo "data" > "$SRC/file.txt"

echo "Writing results to $OUT_FILE" | tee -a "$OUT_FILE"

for ((mask=0; mask< (1<<N); mask++)); do
  args=()
  for ((i=0; i<N; i++)); do
    if ((mask & (1<<i))); then
      args+=("${FLAGS[i]}")
    fi
  done
  combo="${args[*]:-(none)}"
  echo "## Flags: $combo" | tee -a "$OUT_FILE"

  RSYNC_DST="$TMP/rsync_dst"
  OC_DST="$TMP/oc_dst"
  mkdir -p "$RSYNC_DST" "$OC_DST"

  RSYNC_OUT="$TMP/rsync.out"
  RSYNC_ERR="$TMP/rsync.err"
  set +e
  rsync "${args[@]}" "$SRC/" "$RSYNC_DST/" >"$RSYNC_OUT" 2>"$RSYNC_ERR"
  RSYNC_EXIT=$?

  OC_OUT="$TMP/oc.out"
  OC_ERR="$TMP/oc.err"
  "$OC_RSYNC" "${args[@]}" "$SRC/" "$OC_DST/" >"$OC_OUT" 2>"$OC_ERR"
  OC_EXIT=$?
  set -e

  RSYNC_STDOUT="$(grep -v '^$' "$RSYNC_OUT" || true)"
  OC_STDOUT="$(grep -v '^$' "$OC_OUT" || true)"
  RSYNC_STDERR="$(grep -v '^$' "$RSYNC_ERR" || true)"
  OC_STDERR="$(grep -v '^$' "$OC_ERR" || true)"

  echo "rsync exit: $RSYNC_EXIT" | tee -a "$OUT_FILE"
  echo "oc-rsync exit: $OC_EXIT" | tee -a "$OUT_FILE"
  echo "rsync stdout:" | tee -a "$OUT_FILE"
  if [ -n "$RSYNC_STDOUT" ]; then echo "$RSYNC_STDOUT" | tee -a "$OUT_FILE"; fi
  echo "oc-rsync stdout:" | tee -a "$OUT_FILE"
  if [ -n "$OC_STDOUT" ]; then echo "$OC_STDOUT" | tee -a "$OUT_FILE"; fi
  echo "rsync stderr:" | tee -a "$OUT_FILE"
  if [ -n "$RSYNC_STDERR" ]; then echo "$RSYNC_STDERR" | tee -a "$OUT_FILE"; fi
  echo "oc-rsync stderr:" | tee -a "$OUT_FILE"
  if [ -n "$OC_STDERR" ]; then echo "$OC_STDERR" | tee -a "$OUT_FILE"; fi

  if [[ "$RSYNC_EXIT" -ne 0 ]] || [[ "$OC_EXIT" -ne 0 ]] || \
     [[ "$RSYNC_EXIT" -ne "$OC_EXIT" ]] || \
     [[ "$RSYNC_STDOUT" != "$OC_STDOUT" ]] || \
     [[ "$RSYNC_STDERR" != "$OC_STDERR" ]]; then
    echo "!! Difference detected" | tee -a "$OUT_FILE"
    exit 1
  fi

  RSYNC_META="$TMP/rsync.meta"
  OC_META="$TMP/oc.meta"
  rsync -aiXn --delete "$SRC/" "$RSYNC_DST/" >"$RSYNC_META"
  rsync -aiXn --delete "$SRC/" "$OC_DST/" >"$OC_META"

  echo "rsync metadata diff:" | tee -a "$OUT_FILE"
  if [ -s "$RSYNC_META" ]; then cat "$RSYNC_META" | tee -a "$OUT_FILE"; fi
  echo "oc-rsync metadata diff:" | tee -a "$OUT_FILE"
  if [ -s "$OC_META" ]; then cat "$OC_META" | tee -a "$OUT_FILE"; fi

  if [ -s "$RSYNC_META" ] || [ -s "$OC_META" ]; then
    echo "!! Metadata mismatch" | tee -a "$OUT_FILE"
    exit 1
  fi

  echo "== Match" | tee -a "$OUT_FILE"
  echo | tee -a "$OUT_FILE"

  rm -rf "$RSYNC_DST" "$OC_DST" "$RSYNC_OUT" "$OC_OUT" \
         "$RSYNC_ERR" "$OC_ERR" "$RSYNC_META" "$OC_META"
done

