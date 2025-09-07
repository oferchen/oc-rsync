#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"
INTEROP_DIR="$ROOT/tests/interop"
WIRE_DIR="$INTEROP_DIR/wire"
FILELIST_DIR="$INTEROP_DIR/filelists"
VERSIONS=("3.0.9" "3.1.3" "3.4.1")
FLAGS=(--recursive --times --perms)

declare -A RSYNC_SHA256=(
  ["3.0.9"]="30f10f8dd5490d28240d4271bb652b1da7a60b22ed2b9ae28090668de9247c05"
  ["3.1.3"]="55cc554efec5fdaad70de921cd5a5eeb6c29a95524c715f3bbf849235b0800c0"
  ["3.4.1"]="2924bcb3a1ed8b551fc101f740b9f0fe0a202b115027647cf69850d65fd88c52"
)

mkdir -p "$WIRE_DIR" "$FILELIST_DIR"

ensure_build_deps() {
  if ! command -v gcc >/dev/null 2>&1; then
    apt-get update >/dev/null
    apt-get install -y build-essential >/dev/null
  fi
  apt-get update >/dev/null
  apt-get install -y libpopt-dev libzstd-dev zlib1g-dev libacl1-dev libxxhash-dev >/dev/null
}

# Build oc-rsync binary
ensure_build_deps
cargo build --quiet -p oc-rsync --bin oc-rsync

# Ensure sshd exists
if ! command -v sshd >/dev/null 2>&1; then
  echo "Installing openssh-server" >&2
  apt-get update >/dev/null
  apt-get install -y openssh-server >/dev/null
fi

TMP="$(mktemp -d)"
cleanup() {
  if [[ -f "$TMP/sshd.pid" ]]; then
    kill "$(cat "$TMP/sshd.pid")" >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP"
}
trap cleanup EXIT

# SSH server setup
PORT=2222
HOSTKEY="$TMP/ssh_host_key"
CLIENTKEY="$TMP/client_key"
ssh-keygen -q -t rsa -N '' -f "$HOSTKEY"
ssh-keygen -q -t ed25519 -N '' -f "$CLIENTKEY"
cat "$CLIENTKEY.pub" > "$TMP/authorized_keys"
cat <<CFG > "$TMP/sshd_config"
Port $PORT
HostKey $HOSTKEY
PidFile $TMP/sshd.pid
AuthorizedKeysFile $TMP/authorized_keys
PasswordAuthentication no
PermitRootLogin yes
CFG
/usr/sbin/sshd -f "$TMP/sshd_config"
SSH="ssh -p $PORT -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i $CLIENTKEY"

download_rsync() {
  local ver="$1"
  local prefix="$ROOT/target/upstream/$ver"
  local bin="$prefix/bin/rsync"
  if [[ ! -x "$bin" ]]; then
    ensure_build_deps
    mkdir -p "$ROOT/target/upstream"
    pushd "$ROOT/target/upstream" >/dev/null
    tarball="rsync-$ver.tar.gz"
    curl --fail --silent --show-error -L "https://download.samba.org/pub/rsync/src/$tarball" -o "$tarball" \
      || { echo "failed to download $tarball" >&2; return 1; }
    sha="${RSYNC_SHA256[$ver]}"
    if [[ -z "$sha" ]]; then
      echo "missing checksum for rsync $ver" >&2
      exit 1
    fi
    echo "$sha  $tarball" | sha256sum -c -
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

for ver in "${VERSIONS[@]}"; do
  BIN="$(download_rsync "$ver")"
  SRC="$TMP/src-$ver"
  mkdir -p "$SRC"
  echo "data" > "$SRC/file.txt"

  RSYNC_LOG="$TMP/rsync-$ver.log"
  RSYNC_LIST="$TMP/rsync-$ver.list"
  "$BIN" "${FLAGS[@]}" -e "$SSH" "$SRC/" "root@localhost:/tmp/rsync_dst_$ver" >"$RSYNC_LOG" 2>&1
  "$BIN" "${FLAGS[@]}" -n --list-only -e "$SSH" "$SRC/" "root@localhost:/tmp/rsync_dst_$ver" >"$RSYNC_LIST" 2>&1

  OC_RSYNC_LOG="$TMP/oc_rsync-$ver.log"
  OC_RSYNC_LIST="$TMP/oc_rsync-$ver.list"
  "$OC_RSYNC" "${FLAGS[@]}" "$SRC/" "root@localhost:/tmp/oc_rsync_dst_$ver" >"$OC_RSYNC_LOG" 2>&1
  "$OC_RSYNC" --list-only "${FLAGS[@]}" "$SRC/" "root@localhost:/tmp/oc_rsync_dst_$ver" >"$OC_RSYNC_LIST" 2>&1

  if [ "${UPDATE:-0}" = "1" ]; then
    cp "$RSYNC_LOG" "$WIRE_DIR/rsync-$ver.log"
    cp "$RSYNC_LIST" "$FILELIST_DIR/rsync-$ver.txt"
  fi

  diff -u "$WIRE_DIR/rsync-$ver.log" "$OC_RSYNC_LOG"
  diff -u "$FILELIST_DIR/rsync-$ver.txt" "$OC_RSYNC_LIST"

  RSYNC_DST="/tmp/rsync_dst_$ver"
  OC_RSYNC_DST="/tmp/oc_rsync_dst_$ver"

  diff -r "$RSYNC_DST" "$OC_RSYNC_DST"

  META_DIFF=$("$BIN" -acn --delete "$RSYNC_DST/" "$OC_RSYNC_DST/")
  if [[ -n "$META_DIFF" ]]; then
    echo "metadata mismatch for rsync $ver" >&2
    echo "$META_DIFF" >&2
    exit 1
  fi
done
