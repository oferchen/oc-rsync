#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
OC_RSYNC="$ROOT/target/debug/oc-rsync"
INTEROP_DIR="$ROOT/tests/interop"
WIRE_DIR="$INTEROP_DIR/wire"
FILELIST_DIR="$INTEROP_DIR/filelists"
VERSIONS=("3.1.3" "3.2.7" "3.3.0")
FLAGS=(--recursive --times --perms)

mkdir -p "$WIRE_DIR" "$FILELIST_DIR"

# Build oc-rsync binary
cargo build --quiet -p oc-rsync-bin --bin oc-rsync

# Ensure sshd exists
if ! command -v sshd >/dev/null 2>&1; then
  echo "Installing openssh-server" >&2
  apt-get update >/dev/null
  apt-get install -y openssh-server >/dev/null
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

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
trap 'kill $(cat "$TMP/sshd.pid") >/dev/null 2>&1 || true' EXIT
SSH="ssh -p $PORT -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i $CLIENTKEY"

for ver in "${VERSIONS[@]}"; do
  SRC_DIR="$ROOT/rsync-$ver"
  BIN="$SRC_DIR/rsync"
  if [ ! -x "$BIN" ]; then
    echo "rsync $ver not found; attempting download" >&2
    TARBALL="rsync-$ver.tar.gz"
    URL="https://download.samba.org/pub/rsync/src/$TARBALL"
    curl -L "$URL" -o "$TMP/$TARBALL"
    tar -C "$ROOT" -xf "$TMP/$TARBALL"
    (cd "$SRC_DIR" && ./configure >/dev/null && make rsync >/dev/null)
  fi

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
done
