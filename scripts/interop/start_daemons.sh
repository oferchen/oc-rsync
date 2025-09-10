#!/usr/bin/env bash
set -euo pipefail

# Start upstream rsync daemons for interoperability tests. Daemons are bound to
# the loopback interface on sequential ports beginning at INTEROP_PORT_BASE
# (default 44000). The script writes environment variable assignments for each
# daemon to target/upstream/daemons.env and records their PIDs in
# target/upstream/daemons.pids.

ROOT="$(git rev-parse --show-toplevel)"
OUT_DIR="$ROOT/target/upstream"
ENV_FILE="$OUT_DIR/daemons.env"
PID_FILE="$OUT_DIR/daemons.pids"

PORT_BASE=${INTEROP_PORT_BASE:-44000}
NEXT_PORT=$PORT_BASE

VERSIONS=(3.0.9 3.1.3 3.4.1)

mkdir -p "$OUT_DIR"
: > "$ENV_FILE"
: > "$PID_FILE"

for ver in "${VERSIONS[@]}"; do
  bin="$OUT_DIR/rsync-$ver/rsync"
  if [[ ! -x "$bin" ]]; then
    echo "Missing rsync binary for $ver at $bin" >&2
    exit 1
  fi
  port=$NEXT_PORT
  NEXT_PORT=$((NEXT_PORT + 1))
  tmp="$(mktemp -d)"
  cat <<CFG > "$tmp/rsyncd.conf"
uid = 0
gid = 0
use chroot = false
max connections = 4
[mod]
  path = $tmp/root
  read only = false
CFG
  mkdir -p "$tmp/root"
  "$bin" --daemon --no-detach --port "$port" --config "$tmp/rsyncd.conf" &
  pid=$!
  echo "RSYNC_${ver//./_}_PORT=$port" >> "$ENV_FILE"
  echo "$pid" >> "$PID_FILE"
  # allow daemon to start
  for _ in {1..50}; do
    if nc -z 127.0.0.1 "$port" >/dev/null 2>&1; then
      break
    fi
    sleep 0.1
  done
  if ! nc -z 127.0.0.1 "$port" >/dev/null 2>&1; then
    echo "Failed to start daemon on port $port" >&2
    exit 1
  fi
  echo "Started rsync $ver daemon on port $port"
done

echo "Daemon environment written to $ENV_FILE"
