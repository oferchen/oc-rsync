#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"
OUT_DIR="$ROOT/tests/interop"
OUT_FILE="$OUT_DIR/interop-grid.log"

# Build oc-rsync if needed
if [ ! -x "$OC_RSYNC" ]; then
  echo "Building oc-rsync" >&2
  cargo build --quiet -p oc-rsync-bin --bin oc-rsync
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

  RSYNC_ERR="$TMP/rsync.err"
  set +e
  rsync "${args[@]}" "$SRC/" "$RSYNC_DST/" 1>/dev/null 2>"$RSYNC_ERR"
  RSYNC_EXIT=$?

  OC_ERR="$TMP/oc.err"
  "$OC_RSYNC" "${args[@]}" "$SRC/" "$OC_DST/" 1>/dev/null 2>"$OC_ERR"
  OC_EXIT=$?
  set -e

  echo "rsync exit: $RSYNC_EXIT" | tee -a "$OUT_FILE"
  echo "oc-rsync exit: $OC_EXIT" | tee -a "$OUT_FILE"

  RSYNC_LINES="$(grep -v '^$' "$RSYNC_ERR" || true)"
  OC_LINES="$(grep -v '^$' "$OC_ERR" || true)"

  echo "rsync stderr:" | tee -a "$OUT_FILE"
  if [ -n "$RSYNC_LINES" ]; then
    echo "$RSYNC_LINES" | tee -a "$OUT_FILE"
  fi
  echo "oc-rsync stderr:" | tee -a "$OUT_FILE"
  if [ -n "$OC_LINES" ]; then
    echo "$OC_LINES" | tee -a "$OUT_FILE"
  fi

  if [[ "$RSYNC_EXIT" -ne "$OC_EXIT" ]] || [[ "$RSYNC_LINES" != "$OC_LINES" ]]; then
    echo "!! Difference detected" | tee -a "$OUT_FILE"
  else
    echo "== Match" | tee -a "$OUT_FILE"
  fi
  echo | tee -a "$OUT_FILE"

  rm -rf "$RSYNC_DST" "$OC_DST" "$RSYNC_ERR" "$OC_ERR"
done

