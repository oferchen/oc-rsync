#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
MATRIX="$ROOT/tests/interop/run_matrix.sh"
DOC="$ROOT/docs/gaps.md"

mapfile -t matrix_scenarios < <(LIST_SCENARIOS=1 bash "$MATRIX")
mapfile -t doc_scenarios < <(sed -n '/Interop matrix scenarios/,/^##/p' "$DOC" | grep -E '^  - ' | sed -E 's/^  - `([^`]+)`.*$/\1/')

if [[ "${matrix_scenarios[*]}" != "${doc_scenarios[*]}" ]]; then
  echo "docs/gaps.md scenarios do not match tests/interop/run_matrix.sh" >&2
  echo "expected: ${matrix_scenarios[*]}" >&2
  echo "found:    ${doc_scenarios[*]}" >&2
  exit 1
fi
