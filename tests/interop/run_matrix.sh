#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCENARIOS=(
  "base"
  "delete --delete"
  "compress_zlib --compress --compress-choice=zlib"
  "compress_zstd --compress --compress-choice=zstd"
  "filters --include=*.txt --exclude=*"
  "metadata -HAX"
  "partial --partial"
  "resume --partial"
  "vanished"
)

if [[ "${LIST_SCENARIOS:-0}" == "1" ]]; then
  for entry in "${SCENARIOS[@]}"; do
    IFS=' ' read -r name _ <<< "$entry"
    echo "$name"
  done
  exit 0
fi

OC_RSYNC="$ROOT/target/debug/oc-rsync"
if [[ ! -x "$OC_RSYNC" ]]; then
  cargo build --quiet --bin oc-rsync --features="acl xattr"
fi

UPSTREAM="${UPSTREAM_RSYNC:-$(bash "$ROOT/tests/interop/build_upstream.sh")}" 
COMMON_FLAGS=(--archive --acls --xattrs)

create_tree() {
  local dir="$1"
  mkdir -p "$dir/sub"
  echo data > "$dir/file.txt"
  echo temp > "$dir/sub/temp.log"
  ln -s file.txt "$dir/link"
  touch "$dir/hard1"
  ln "$dir/hard1" "$dir/hard2"
  setfattr -n user.test -v 123 "$dir/file.txt" 2>/dev/null || true
  setfacl -m u::rwx "$dir/file.txt" 2>/dev/null || true
}

setup_daemon() {
  local server_bin="$1"
  local tmp="$(mktemp -d)"
  local port=$(shuf -i 40000-49999 -n 1)
  cat <<CFG > "$tmp/rsyncd.conf"
uid = root
gid = root
use chroot = false
max connections = 4
[mod]
  path = $tmp/root
  read only = false
CFG
  mkdir -p "$tmp/root"
  "$server_bin" --daemon --no-detach --port "$port" --config "$tmp/rsyncd.conf" &
  local pid=$!
  printf '%s\t%s\t%s\n' "$port" "$tmp/root" "$pid"
}

verify_tree() {
  local src="$1" dst="$2"
  $UPSTREAM -an --delete "$src/" "$dst/" | tee /tmp/verify.log
  if [[ -s /tmp/verify.log ]]; then
    echo "Trees differ" >&2
    exit 1
  fi
}

for entry in "${SCENARIOS[@]}"; do
  IFS=' ' read -r name extra <<< "$entry"
  extra=($extra)
  src="$(mktemp -d)"
  create_tree "$src"
  IFS=$'\t' read -r port rootdir pid < <(setup_daemon "$UPSTREAM")
  case "$name" in
    delete) touch "$rootdir/stale.txt" ;;
    resume) dd if=/dev/zero of="$src/big.bin" bs=1M count=20 >/dev/null ;;
    partial) dd if=/dev/zero of="$src/big.bin" bs=1M count=20 >/dev/null ;;
    vanished) (sleep 0.1 && rm -f "$src/file.txt") & ;;
  esac
  if [[ "$name" == resume ]]; then
    timeout 1 "$OC_RSYNC" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >/dev/null || true
  fi
  set +e
  "$OC_RSYNC" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" > /tmp/stdout 2> /tmp/stderr
  status=$?
  set -e
  if [[ "$name" == vanished ]]; then
    [[ "$status" -ne 0 ]] || { echo "expected failure" >&2; exit 1; }
  else
    verify_tree "$src" "$rootdir"
  fi
  kill "$pid" >/dev/null 2>&1 || true
  rm -rf "$src" "$rootdir"
  echo "scenario $name ok"
done
