#!/usr/bin/env bash
set -euo pipefail

# Build known upstream rsync releases and verify their checksums before use.
# By default builds rsync 3.0.9, 3.1.3, and 3.4.1. Passing version arguments
# limits the build to that subset. Each invocation prints the path to the
# resulting rsync binary for every version built.

ROOT="$(git rev-parse --show-toplevel)"
OUT_DIR="$ROOT/target/upstream"

VERSIONS=("3.0.9" "3.1.3" "3.4.1")
if [[ $# -gt 0 ]]; then
  VERSIONS=("$@")
fi

declare -A SHA256=(
  ["3.0.9"]=30f10f8dd5490d28240d4271bb652b1da7a60b22ed2b9ae28090668de9247c05
  ["3.1.3"]=55cc554efec5fdaad70de921cd5a5eeb6c29a95524c715f3bbf849235b0800c0
  ["3.4.1"]=2924bcb3a1ed8b551fc101f740b9f0fe0a202b115027647cf69850d65fd88c52
)

ensure_build_deps() {
  local apt="apt-get"
  if command -v sudo >/dev/null 2>&1; then
    apt="sudo apt-get"
  fi
  if ! command -v gcc >/dev/null 2>&1; then
    $apt update >/dev/null
    $apt install -y build-essential >/dev/null
  fi
  $apt update >/dev/null
  $apt install -y libpopt-dev libzstd-dev zlib1g-dev libacl1-dev libxxhash-dev >/dev/null
}

ensure_build_deps

mkdir -p "$OUT_DIR"

for ver in "${VERSIONS[@]}"; do
  prefix="$OUT_DIR/rsync-$ver"
  bin="$prefix/rsync"
  if [[ -x "$bin" ]]; then
    echo "$bin"
    continue
  fi
  tarball="rsync-$ver.tar.gz"
  url="https://download.samba.org/pub/rsync/src/$tarball"
  pushd "$OUT_DIR" >/dev/null
  curl -fL --retry 3 -o "$tarball" "$url"
  sha256="${SHA256[$ver]}"
  if [[ -z "$sha256" ]]; then
    echo "Unknown SHA256 for rsync $ver" >&2
    exit 1
  fi
  echo "$sha256  $tarball" | sha256sum -c -
  tar xzf "$tarball"
  pushd "rsync-$ver" >/dev/null
  ./configure >/dev/null
  make -j"$(nproc)" >/dev/null
  popd >/dev/null
  popd >/dev/null
  mv "$OUT_DIR/rsync-$ver/rsync" "$bin" 2>/dev/null || true
  echo "$bin"
  done
