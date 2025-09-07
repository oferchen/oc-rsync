#!/usr/bin/env bash
set -euo pipefail

# Build a known upstream rsync release and verify its checksum before use.
ROOT="$(git rev-parse --show-toplevel)"
OUT_DIR="$ROOT/target/upstream"
VER=3.4.1
SHA256="2924bcb3a1ed8b551fc101f740b9f0fe0a202b115027647cf69850d65fd88c52"
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

tarball="rsync-$VER.tar.gz"
curl -L -O "https://download.samba.org/pub/rsync/src/$tarball"
echo "$SHA256  $tarball" | sha256sum -c -
tar xzf "$tarball"
cd "rsync-$VER"
./configure >/dev/null
make -j"$(nproc)" >/dev/null

ln -sf "$(pwd)/rsync" "$OUT_DIR/rsync"

echo "$OUT_DIR/rsync"
