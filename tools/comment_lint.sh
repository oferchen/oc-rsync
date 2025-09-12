#!/usr/bin/env bash
set -euo pipefail

fail=0

while IFS= read -r -d '' file; do
    [[ $file == src/* || $file == crates/* ]] || continue
    header="$(head -n 1 "$file")"
    if [[ $header != "// crates/"* && $header != "// src/"* ]]; then
        echo "$file: missing header" >&2
        fail=1
        continue
    fi
    if tail -n +2 "$file" | grep -E '^[[:space:]]*//' | grep -vE '^[[:space:]]*///' >/dev/null; then
        echo "$file: contains // comments" >&2
        fail=1
    fi
done < <(git ls-files -z -- '*.rs')

exit $fail
