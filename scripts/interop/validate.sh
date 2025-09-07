#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OUT_DIR="$ROOT/tests/interop/streams"
OC_RSYNC="$ROOT/target/debug/oc-rsync"
UPSTREAM="${UPSTREAM_RSYNC:-rsync}"

# Build oc-rsync if the binary is missing so we can determine its version
if [ ! -x "$OC_RSYNC" ]; then
  cargo build --quiet -p oc-rsync --bin oc-rsync
fi

OC_VER="$($OC_RSYNC --version 2>/dev/null | head -n1 | awk '{print $2}')"
UP_VER="$($UPSTREAM --version | head -n1 | awk '{print $3}')"

FLAGS=(--archive --compress --delete)
N=${#FLAGS[@]}

for ((mask=0; mask< (1<<N); mask++)); do
  args=()
  for ((i=0; i<N; i++)); do
    if ((mask & (1<<i))); then
      args+=("${FLAGS[i]}")
    fi
  done
  combo="${args[*]:-(none)}"
  prefix="${combo// /_}"

  diff -u "$OUT_DIR/${prefix}.rsync-${UP_VER}.stdout" "$OUT_DIR/${prefix}.oc-rsync-${OC_VER}.stdout"
  diff -u "$OUT_DIR/${prefix}.rsync-${UP_VER}.stderr" "$OUT_DIR/${prefix}.oc-rsync-${OC_VER}.stderr"

  rsync_exit=$(cat "$OUT_DIR/${prefix}.rsync-${UP_VER}.exit")
  oc_exit=$(cat "$OUT_DIR/${prefix}.oc-rsync-${OC_VER}.exit")
  if [[ "$rsync_exit" -ne "$oc_exit" ]]; then
    echo "exit codes differ for $combo: rsync=$rsync_exit oc-rsync=$oc_exit" >&2
    exit 1
  fi

done
