#!/usr/bin/env bash
set -euo pipefail

fail=0
pattern='todo!|unimplemented!|FIXME|XXX|panic!\("(TODO|FIXME|XXX|unimplemented)"'

while IFS= read -r -d '' file; do
    offenses=$(grep -nE "$pattern" "$file" | grep -v '^1:' || true)
    if [[ -n $offenses ]]; then
        printf '%s\n' "$offenses" | sed "s/^/$file:/" >&2
        fail=1
    fi
done < <(git ls-files -z -- '*.rs')

exit $fail
