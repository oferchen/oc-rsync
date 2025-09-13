#!/usr/bin/env bash
set -euo pipefail

fail=0

# Layer order
order=(checksums compress filters walk meta protocol engine logging core transport daemon cli)

# Return index of crate in layer order or -1
idx() {
    local name=$1
    for i in "${!order[@]}"; do
        if [[ "${order[$i]}" == "$name" ]]; then
            echo "$i"
            return
        fi
    done
    echo -1
}

metadata=$(cargo metadata --no-deps --format-version 1)

while IFS= read -r -d '' manifest; do
    crate="$(basename "$(dirname "$manifest")")"
    crate_idx=$(idx "$crate")
    (( crate_idx >= 0 )) || continue
    deps=$(printf '%s' "$metadata" | jq -r --arg name "$crate" '.packages[] | select(.name==$name) | .dependencies[]?.name')
    for dep in $deps; do
        dep_idx=$(idx "$dep")
        (( dep_idx >= 0 )) || continue
        if (( dep_idx > crate_idx )); then
            echo "$crate -> $dep" >&2
            fail=1
        fi
    done
done < <(git ls-files -z crates/*/Cargo.toml)

exit $fail
