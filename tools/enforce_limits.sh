#!/usr/bin/env bash
set -euo pipefail

max=${MAX_RUST_LINES:-600}
fail=0

while IFS= read -r -d '' file; do
    case "$file" in
        src/*|crates/*/src/*) ;; 
        *) continue ;;
    esac
    lines=$(wc -l < "$file")
    if (( lines > max )); then
        echo "$file: $lines lines (max $max)" >&2
        fail=1
    fi
    header="$(head -n 1 "$file")"
    if [[ $header != "// crates/"* && $header != "// src/"* ]]; then
        echo "$file: missing header" >&2
        fail=1
    fi
done < <(git ls-files -z -- '*.rs')

exit $fail
