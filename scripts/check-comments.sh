#!/usr/bin/env bash
set -euo pipefail

files="$@"
if [ $# -eq 0 ]; then
    files=$(git ls-files '*.rs')
fi
cargo run --quiet --bin strip-rs-comments -- --check $files
