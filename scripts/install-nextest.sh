#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo-nextest >/dev/null 2>&1; then
  echo "cargo-nextest not found, installing..."
  cargo install cargo-nextest --locked
else
  echo "cargo-nextest already installed"
fi

cargo nextest --version
