#!/usr/bin/env bash
set -euo pipefail

fail=0

# Layer mapping
declare -A layer=(
    [checksums]=0
    [compress]=0
    [filters]=0
    [meta]=0
    [protocol]=0
    [transport]=1
    [engine]=1
    [logging]=1
    [cli]=2
)

metadata=$(cargo metadata --no-deps --format-version 1)

while IFS= read -r -d '' manifest; do
    crate="$(basename "$(dirname "$manifest")")"
    [[ -n ${layer[$crate]+_} ]] || continue
    deps=$(printf '%s' "$metadata" | jq -r --arg name "$crate" '.packages[] | select(.name==$name) | .dependencies[]?.name')
    for dep in $deps; do
        [[ -n ${layer[$dep]+_} ]] || continue
        if (( layer[$dep] > layer[$crate] )); then
            echo "$crate -> $dep" >&2
            fail=1
        fi
    done
done < <(git ls-files -z crates/*/Cargo.toml)

exit $fail
