#!/usr/bin/env bash
set -euo pipefail

files=$(git ls-files '*.rs' | grep -v '^crates/' | grep -v '^tests/')
cargo run --quiet --bin strip-comments -- --check $files
