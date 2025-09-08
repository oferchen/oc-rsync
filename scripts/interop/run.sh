#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OUT_DIR="$ROOT/tests/interop/streams"
OC_RSYNC="$ROOT/target/debug/oc-rsync"
UPSTREAM="${UPSTREAM_RSYNC:-rsync}"
FLAGS=(--archive --compress --delete)
N=${#FLAGS[@]}

cmp_trees() {
  local a="$1" b="$2" diff_file="$3"
  rsync -an --delete -rlptgoD --acls --xattrs "$a/" "$b/" | tee "$diff_file"
  if [[ -s "$diff_file" ]]; then
    echo "Trees differ" >&2
    return 1
  fi
}

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

# Build oc-rsync if the binary is missing
if [ ! -x "$OC_RSYNC" ]; then
  ensure_build_deps() {
    if ! command -v gcc >/dev/null 2>&1; then
      apt-get update >/dev/null
      apt-get install -y build-essential >/dev/null
    fi
    apt-get update >/dev/null
    apt-get install -y libpopt-dev libzstd-dev zlib1g-dev libacl1-dev libxxhash-dev >/dev/null
  }
  ensure_build_deps
  cargo build --quiet -p oc-rsync --bin oc-rsync
fi

OC_VER="$($OC_RSYNC --version 2>/dev/null | head -n1 | awk '{print $2}')"
UP_VER="$($UPSTREAM --version | head -n1 | awk '{print $3}')"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

SRC="$TMP/src"
mkdir -p "$SRC"
echo data > "$SRC/file.txt"

for ((mask=0; mask< (1<<N); mask++)); do
  args=()
  for ((i=0; i<N; i++)); do
    if ((mask & (1<<i))); then
      args+=("${FLAGS[i]}")
    fi
  done
  combo="${args[*]:-(none)}"
  prefix="${combo// /_}"

  RSYNC_DST="$TMP/rsync_dst"
  OC_DST="$TMP/oc_dst"
  mkdir -p "$RSYNC_DST" "$OC_DST"

  set +e
  "$UPSTREAM" "${args[@]}" "$SRC/" "$RSYNC_DST/" >"$OUT_DIR/${prefix}.rsync-${UP_VER}.stdout" 2>"$OUT_DIR/${prefix}.rsync-${UP_VER}.stderr"
  echo $? >"$OUT_DIR/${prefix}.rsync-${UP_VER}.exit"
  set -e

  set +e
  "$OC_RSYNC" "${args[@]}" "$SRC/" "$OC_DST/" >"$OUT_DIR/${prefix}.oc-rsync-${OC_VER}.stdout" 2>"$OUT_DIR/${prefix}.oc-rsync-${OC_VER}.stderr"
  echo $? >"$OUT_DIR/${prefix}.oc-rsync-${OC_VER}.exit"
  set -e

  RSYNC_STDOUT="$OUT_DIR/${prefix}.rsync-${UP_VER}.stdout"
  RSYNC_STDERR="$OUT_DIR/${prefix}.rsync-${UP_VER}.stderr"
  RSYNC_EXIT="$OUT_DIR/${prefix}.rsync-${UP_VER}.exit"
  OC_STDOUT="$OUT_DIR/${prefix}.oc-rsync-${OC_VER}.stdout"
  OC_STDERR="$OUT_DIR/${prefix}.oc-rsync-${OC_VER}.stderr"
  OC_EXIT="$OUT_DIR/${prefix}.oc-rsync-${OC_VER}.exit"

  diff "$RSYNC_STDOUT" "$OC_STDOUT"
  diff "$RSYNC_STDERR" "$OC_STDERR"
  diff "$RSYNC_EXIT" "$OC_EXIT"

  DIR_DIFF="$OUT_DIR/${prefix}.dir.diff"
  cmp_trees "$RSYNC_DST" "$OC_DST" "$DIR_DIFF"

  rm -rf "$RSYNC_DST" "$OC_DST"
done
