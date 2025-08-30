#!/usr/bin/env bash
set -euo pipefail

files=$(git ls-files '*.rs' | grep -v '^crates/')
cargo run --quiet --bin strip-comments -- --check $files
