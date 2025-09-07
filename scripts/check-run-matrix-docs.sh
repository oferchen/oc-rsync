#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
MATRIX="$ROOT/scripts/interop.sh"
DOC="$ROOT/docs/gaps.md"

if grep -q '^SCENARIOS=(' "$MATRIX"; then
  mapfile -t matrix_scenarios < <(grep '^SCENARIOS=(' "$MATRIX" | sed -E 's/.*\(([^)]*)\).*/\1/' | tr ' ' '\n')
  mapfile -t doc_scenarios < <(sed -n '/Interop matrix scenarios/,/^##/p' "$DOC" | grep -E '^  - ' | sed -E 's/^  - `([^`]+)`.*$/\1/')
  if [[ "${matrix_scenarios[*]}" != "${doc_scenarios[*]}" ]]; then
    echo "docs/gaps.md scenarios do not match scripts/interop.sh" >&2
    echo "expected: ${matrix_scenarios[*]}" >&2
    echo "found:    ${doc_scenarios[*]}" >&2
    exit 1
  fi
else
  echo "scripts/interop.sh does not define SCENARIOS; skipping verification" >&2
fi
