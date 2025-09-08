#!/usr/bin/env bash
set -euo pipefail

# Run interop scenarios against multiple upstream rsync releases with
# verified source tarballs.
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
if [[ -n ${SCENARIOS_OVERRIDE:-} ]]; then
  readarray -t SCENARIOS <<< "$SCENARIOS_OVERRIDE"
fi

RSYNC_VERSIONS=(3.0.9 3.1.3 3.4.1)
if [[ -n ${RSYNC_VERSIONS_OVERRIDE:-} ]]; then
  readarray -t RSYNC_VERSIONS <<< "$RSYNC_VERSIONS_OVERRIDE"
fi

declare -A RSYNC_SHA256=(
  [3.0.9]=30f10f8dd5490d28240d4271bb652b1da7a60b22ed2b9ae28090668de9247c05
  [3.1.3]=55cc554efec5fdaad70de921cd5a5eeb6c29a95524c715f3bbf849235b0800c0
  [3.4.1]=2924bcb3a1ed8b551fc101f740b9f0fe0a202b115027647cf69850d65fd88c52
)

# Base port for rsync daemons. Override with INTEROP_PORT_BASE to avoid
# collisions when running tests in parallel environments. Ports are
# assigned sequentially for determinism across runs.
PORT_BASE=${INTEROP_PORT_BASE:-43000}
NEXT_PORT=$PORT_BASE

if [[ "${LIST_SCENARIOS:-0}" == "1" ]]; then
  for entry in "${SCENARIOS[@]}"; do
    IFS=' ' read -r name _ <<< "$entry"
    echo "$name"
  done
  exit 0
fi

OC_RSYNC="$ROOT/target/debug/oc-rsync"

ensure_build_deps() {
  local apt="apt-get"
  if command -v sudo >/dev/null 2>&1; then
    apt="sudo apt-get"
  fi
  if ! command -v gcc >/dev/null 2>&1; then
    $apt update >/dev/null
    $apt install -y build-essential >/dev/null
  fi
  $apt update >/dev/null
  $apt install -y libpopt-dev libzstd-dev zlib1g-dev libacl1-dev libxxhash-dev >/dev/null
}

if [[ ! -x "$OC_RSYNC" ]]; then
  ensure_build_deps
  cargo build --quiet --bin oc-rsync --features="acl xattr"
fi

download_rsync() {
  local ver="$1"
  local prefix="$ROOT/target/upstream/$ver"
  local bin="$prefix/bin/rsync"
  if [[ ! -x "$bin" ]]; then
    ensure_build_deps
    mkdir -p "$ROOT/target/upstream"
    pushd "$ROOT/target/upstream" >/dev/null
    tarball="rsync-$ver.tar.gz"
    curl -L "https://download.samba.org/pub/rsync/src/$tarball" -o "$tarball"
    sha256="${RSYNC_SHA256[$ver]}"
    if [[ -z "$sha256" ]]; then
      echo "Unknown SHA256 for rsync $ver" >&2
      exit 1
    fi
    echo "$sha256  $tarball" | sha256sum -c -
    tar xzf "$tarball"
    pushd "rsync-$ver" >/dev/null
    ./configure --prefix="$prefix" >/dev/null
    make -j"$(nproc)" >/dev/null
    make install >/dev/null
    popd >/dev/null
    popd >/dev/null
  fi
  printf '%s' "$bin"
}

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
  local port=$NEXT_PORT
  NEXT_PORT=$((NEXT_PORT + 1))
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
  for _ in {1..50}; do
    if nc -z localhost "$port" >/dev/null 2>&1; then
      printf '%s\t%s\t%s\n' "$port" "$tmp/root" "$pid"
      return 0
    fi
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      wait "$pid" >/dev/null 2>&1 || true
      echo "daemon exited before opening port" >&2
      return 1
    fi
    sleep 0.1
  done
  echo "timeout waiting for port $port" >&2
  kill "$pid" >/dev/null 2>&1 || true
  wait "$pid" >/dev/null 2>&1 || true
  return 1
}

verify_tree() {
  local src="$1" dst="$2"
  $UPSTREAM -an --delete --acls --xattrs "$src/" "$dst/" | tee /tmp/verify.log
  if [[ -s /tmp/verify.log ]]; then
    echo "Trees differ" >&2
    exit 1
  fi
}

for ver in "${RSYNC_VERSIONS[@]}"; do
  echo "Testing against rsync $ver"
  UPSTREAM="$(download_rsync "$ver")"
  COMMON_FLAGS=(--archive --acls --xattrs)

  for entry in "${SCENARIOS[@]}"; do
    IFS=' ' read -r name extra <<< "$entry"
    extra=($extra)

    # oc-rsync client against upstream daemon
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
      timeout 1 "$OC_RSYNC" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >/dev/null 2>&1 || true
    fi
    set +e
    "$OC_RSYNC" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" > /tmp/oc.stdout 2> /tmp/oc.stderr
    status_oc=$?
    set -e
    if [[ "$name" == vanished ]]; then
      [[ "$status_oc" -ne 0 ]] || { echo "expected failure" >&2; exit 1; }
    else
      verify_tree "$src" "$rootdir"
    fi
    kill "$pid" >/dev/null 2>&1 || true
    rm -rf "$rootdir" "$src"

    # upstream client against oc-rsync daemon
    src="$(mktemp -d)"
    create_tree "$src"
    IFS=$'\t' read -r port rootdir pid < <(setup_daemon "$OC_RSYNC")
    case "$name" in
      delete) touch "$rootdir/stale.txt" ;;
      resume) dd if=/dev/zero of="$src/big.bin" bs=1M count=20 >/dev/null ;;
      partial) dd if=/dev/zero of="$src/big.bin" bs=1M count=20 >/dev/null ;;
      vanished) (sleep 0.1 && rm -f "$src/file.txt") & ;;
    esac
    if [[ "$name" == resume ]]; then
      timeout 1 "$UPSTREAM" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >/dev/null 2>&1 || true
    fi
    set +e
    "$UPSTREAM" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" > /tmp/up.stdout 2> /tmp/up.stderr
    status_up=$?
    set -e
    if [[ "$name" == vanished ]]; then
      [[ "$status_up" -ne 0 ]] || { echo "expected failure" >&2; exit 1; }
    else
      verify_tree "$src" "$rootdir"
    fi
    kill "$pid" >/dev/null 2>&1 || true
    rm -rf "$rootdir" "$src"

    # compare outputs and exit codes
    if ! diff -u /tmp/oc.stdout /tmp/up.stdout; then
      echo "stdout mismatch for $name" >&2
      exit 1
    fi
    if ! diff -u /tmp/oc.stderr /tmp/up.stderr; then
      echo "stderr mismatch for $name" >&2
      exit 1
    fi
    if [[ $status_oc -ne $status_up ]]; then
      echo "exit codes differ: oc $status_oc upstream $status_up" >&2
      exit 1
    fi

    echo "scenario $name ok"
  done
done

