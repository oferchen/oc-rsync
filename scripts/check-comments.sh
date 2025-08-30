#!/usr/bin/env bash
set -euo pipefail

files=$(git ls-files '*.rs')
cargo run --quiet --bin strip-rs-comments -- --check $files

