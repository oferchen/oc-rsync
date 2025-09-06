#!/usr/bin/env bash
set -euo pipefail

missing=()

if [[ -z "${CURRENT_YEAR:-}" ]]; then
  echo "Warning: CURRENT_YEAR environment variable not set; build.rs will use the system year." >&2
fi

if ! command -v ld >/dev/null 2>&1; then
  missing+=("ld (linker from binutils)")
fi

if ! ldconfig -p 2>/dev/null | grep -q libzstd; then
  missing+=("libzstd (install libzstd-dev)")
fi

if ! ldconfig -p 2>/dev/null | grep -q 'libz\.so'; then
  missing+=("zlib (install zlib1g-dev)")
fi

missing_optional=()

if ! ldconfig -p 2>/dev/null | grep -q libacl; then
  missing_optional+=("libacl (install libacl1-dev or disable the 'acl' feature)")
fi

if ((${#missing[@]})); then
  echo "Error: missing required build dependencies:" >&2
  for dep in "${missing[@]}"; do
    echo "  - $dep" >&2
  done
  exit 1
fi

if ((${#missing_optional[@]})); then
  echo "Warning: missing optional build dependencies:" >&2
  for dep in "${missing_optional[@]}"; do
    echo "  - $dep" >&2
  done
fi

echo "All required linkers and libraries are present."
