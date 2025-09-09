#!/usr/bin/env bash
set -euo pipefail

readarray -d '' -t files < <(git ls-files -z '*.rs')
cargo run --quiet -p xtask --bin comment_lint -- "${files[@]}"
