#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OUT_DIR="$ROOT/tests/interop/streams"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

FLAGS=(--archive --compress --delete)
N=${#FLAGS[@]}

# List of upstream versions to exercise. Override with RSYNC_VERSIONS env var
# (space-separated).
VERSIONS=(3.0.9 3.1.3 3.4.1)
if [[ -n ${RSYNC_VERSIONS:-} ]]; then
  read -r -a VERSIONS <<<"$RSYNC_VERSIONS"
fi

cmp_trees() {
  local a="$1" b="$2" diff_file="$3"
  local LC_ALL=C
  : >"$diff_file"
  local paths tmp
  tmp="$(mktemp)"
  find "$a" "$b" -mindepth 1 -printf "%P\0" | sort -z | uniq -z >"$tmp"
  while IFS= read -r -d '' paths; do
    local stat_a stat_b xattr_a xattr_b acl_a acl_b
    if ! stat_a=$(stat -c "%f %u %g %s %Y" "$a/$paths" 2>/dev/null); then
      stat_a="MISSING"
    fi
    if ! stat_b=$(stat -c "%f %u %g %s %Y" "$b/$paths" 2>/dev/null); then
      stat_b="MISSING"
    fi
    xattr_a=$(getfattr -d -m - "$a/$paths" 2>/dev/null | sed '1d' | sort | tr '\n' ' ' || true)
    xattr_b=$(getfattr -d -m - "$b/$paths" 2>/dev/null | sed '1d' | sort | tr '\n' ' ' || true)
    acl_a=$(getfacl --absolute-names --numeric "$a/$paths" 2>/dev/null | sed '1d' | sort | tr '\n' ' ' || true)
    acl_b=$(getfacl --absolute-names --numeric "$b/$paths" 2>/dev/null | sed '1d' | sort | tr '\n' ' ' || true)
    if [[ "$stat_a" != "$stat_b" || "$xattr_a" != "$xattr_b" || "$acl_a" != "$acl_b" ]]; then
      printf "%s\n  stat: %s\n  stat: %s\n  xattr: %s\n  xattr: %s\n  acl: %s\n  acl: %s\n" \
        "$paths" "$stat_a" "$stat_b" "$xattr_a" "$xattr_b" "$acl_a" "$acl_b" >>"$diff_file"
    fi
  done <"$tmp"
  rm -f "$tmp"
  if [[ -s "$diff_file" ]]; then
    echo "Trees differ" >&2
    return 1
  fi
}

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

# Ensure required tools are available
ensure_tools() {
  local apt="apt-get"
  if command -v sudo >/dev/null 2>&1; then
    apt="sudo apt-get"
  fi
  if ! command -v getfattr >/dev/null 2>&1 || ! command -v getfacl >/dev/null 2>&1; then
    $apt update >/dev/null
    $apt install -y attr acl >/dev/null
  fi
}
ensure_tools


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

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

SRC="$TMP/src"
mkdir -p "$SRC"
echo data > "$SRC/file.txt"

for ver in "${VERSIONS[@]}"; do
  UPSTREAM="$("$ROOT/scripts/interop/build_upstream.sh" "$ver" | tail -n1)"
  if [[ ! -x "$UPSTREAM" ]]; then
    echo "Failed to build rsync $ver" >&2
    exit 1
  fi
  UP_VER="$($UPSTREAM --version | head -n1 | awk '{print $3}')"

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
done
