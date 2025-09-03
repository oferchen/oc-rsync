#!/usr/bin/env bash
set -euo pipefail

VERSION="3.4.1"
TARBALL="rsync-${VERSION}.tar.gz"
URL="https://download.samba.org/pub/rsync/src/${TARBALL}"
SHA256="2924bcb3a1ed8b551fc101f740b9f0fe0a202b115027647cf69850d65fd88c52"

if [ ! -f "$TARBALL" ]; then
  echo "Downloading $TARBALL..."
  curl --fail --location --silent --show-error "$URL" -o "$TARBALL"
fi

echo "Verifying $TARBALL..."
echo "$SHA256  $TARBALL" | sha256sum -c -

if [ ! -d "rsync-${VERSION}" ]; then
  echo "Extracting $TARBALL..."
  tar -xf "$TARBALL"
fi
