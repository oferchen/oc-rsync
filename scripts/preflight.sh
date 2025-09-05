#!/usr/bin/env bash
set -euo pipefail

missing=()

if ! command -v ld >/dev/null 2>&1; then
  missing+=("ld (linker from binutils)")
fi

if ! ldconfig -p 2>/dev/null | grep -q libzstd; then
  missing+=("libzstd (install libzstd-dev)")
fi

if ! ldconfig -p 2>/dev/null | grep -q 'libz\.so'; then
  missing+=("zlib (install zlib1g-dev)")
fi

if ! ldconfig -p 2>/dev/null | grep -q libacl; then
  missing+=("libacl (install libacl1-dev)")
fi

if ((${#missing[@]})); then
  echo "Error: missing required build dependencies:" >&2
  for dep in "${missing[@]}"; do
    echo "  - $dep" >&2
  done
  exit 1
fi

echo "All required linkers and libraries are present."
