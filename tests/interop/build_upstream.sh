#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OUT_DIR="$ROOT/target/upstream"
VER=3.4.1
PREFIX="$OUT_DIR/rsync-$VER"
BIN="$PREFIX/rsync"

if [[ -x "$BIN" ]]; then
  echo "$BIN"
  exit 0
fi

mkdir -p "$OUT_DIR"
cd "$OUT_DIR"

# Install build dependencies if missing
if ! command -v gcc >/dev/null; then
  apt-get update
  apt-get install -y build-essential >/dev/null
fi
apt-get update
apt-get install -y libpopt-dev libzstd-dev zlib1g-dev libacl1-dev libxxhash-dev >/dev/null

curl -L -O "https://download.samba.org/pub/rsync/src/rsync-$VER.tar.gz"
tar xzf "rsync-$VER.tar.gz"
cd "rsync-$VER"
./configure >/dev/null
make -j"$(nproc)" >/dev/null

ln -sf "$(pwd)/rsync" "$OUT_DIR/rsync"

echo "$OUT_DIR/rsync"
