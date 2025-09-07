#!/usr/bin/env bash
set -euo pipefail

if [ $# -eq 0 ]; then
    readarray -d '' -t files < <(git ls-files -z '*.rs')
else
    files=("$@")
fi
cargo run --quiet -p xtask --bin comment_lint -- "${files[@]}"
