#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"

# Ensure binary is built
cargo build --quiet --bin oc-rsync

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src/keep/sub" "$TMP/src/keep/tmp" "$TMP/src/skip" "$TMP/oc_rsync_dst"

# Populate source tree

echo keep > "$TMP/src/keep/file.txt"
echo sub > "$TMP/src/keep/sub/file.md"
echo tmp > "$TMP/src/keep/tmp/file.tmp"
echo skip > "$TMP/src/skip/file.txt"
echo root > "$TMP/src/root.tmp"
echo core > "$TMP/src/core"
echo obj > "$TMP/src/foo.o"
echo debug > "$TMP/src/debug.log"
echo info > "$TMP/src/info.log"
cat > "$TMP/src/.rsync-filter" <<'EOF'
+ keep/tmp/file.tmp
- *.tmp
EOF
cat > "$TMP/src/.gitignore" <<'EOF'
- *.log
EOF
echo info > "$TMP/src/keep/info.log"

# Run oc-rsync
oc_rsync_raw=$("$OC_RSYNC" --recursive \
  --filter=': /.rsync-filter' \
  --filter=': /.gitignore' \
  --filter='- .rsync-filter' \
  --filter='- .gitignore' \
  --filter='+ core' \
  --filter='-C' \
  --filter='S debug.log' \
  --filter='- skip/' \
  --filter='+ keep/***' \
  --filter='+ *.md' \
  --filter='- *' \
  "$TMP/src/" "$TMP/oc_rsync_dst" 2>&1)
oc_rsync_status=$?
oc_rsync_output=$(echo "$oc_rsync_raw" | grep -v -e 'recursive mode enabled' || true)

expected_output=$(cat "$ROOT/tests/golden/filter_rule_precedence/output.txt")
expected_status=$(cat "$ROOT/tests/golden/filter_rule_precedence/exit_code")

if [ "$expected_status" -ne "$oc_rsync_status" ]; then
  echo "Exit codes differ: expected=$expected_status oc-rsync=$oc_rsync_status" >&2
  exit 1
fi

if [ "$expected_output" != "$oc_rsync_output" ]; then
  echo "Outputs differ" >&2
  diff -u <(printf "%s" "$expected_output") <(printf "%s" "$oc_rsync_output") >&2 || true
  exit 1
fi

if ! diff -r "$ROOT/tests/golden/filter_rule_precedence/dst" "$TMP/oc_rsync_dst" >/dev/null; then
  echo "Directory trees differ" >&2
  diff -r "$ROOT/tests/golden/filter_rule_precedence/dst" "$TMP/oc_rsync_dst" >&2 || true
  exit 1
fi
