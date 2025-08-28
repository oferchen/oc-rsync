#!/usr/bin/env bash
set -euo pipefail

# Run a matrix of rsync 3.2.x client/server combinations over SSH and rsync://
# connections. Golden directory trees are stored under tests/interop/golden
# using the layout:
#   golden/<client>_<server>_<transport>
# When UPDATE=1 is set the goldens are regenerated.

ROOT="$(git rev-parse --show-toplevel)"
GOLDEN="$ROOT/tests/interop/golden"
CLIENT_VERSIONS=("3.2.0" "3.2.7")
SERVER_VERSIONS=("3.2.0" "3.2.7")
TRANSPORTS=("ssh" "rsync")

# Options used for all transfers. -a implies -rtgoD, we add -A and -X for ACLs
# and xattrs.
COMMON_FLAGS=(--archive --acls --xattrs)

mkdir -p "$GOLDEN"

fetch_rsync() {
  local ver="$1"
  local dir="$ROOT/rsync-$ver"
  local bin="$dir/rsync"
  if [[ ! -x "$bin" ]]; then
    echo "Fetching rsync $ver" >&2
    local tar="rsync-$ver.tar.gz"
    local url="https://download.samba.org/pub/rsync/src/$tar"
    curl -Ls "$url" -o "$tar"
    tar -xf "$tar"
    (cd "$dir" && ./configure >/dev/null && make rsync >/dev/null)
  fi
  echo "$bin"
}

setup_ssh() {
  local server_bin="$1"
  local tmp
  tmp="$(mktemp -d)"
  local port=2222
  local hostkey="$tmp/ssh_host_key"
  local clientkey="$tmp/client_key"
  ssh-keygen -q -t rsa -N '' -f "$hostkey" >/dev/null
  ssh-keygen -q -t ed25519 -N '' -f "$clientkey" >/dev/null
  cat "$clientkey.pub" > "$tmp/authorized_keys"
  cat <<CFG > "$tmp/sshd_config"
Port $port
HostKey $hostkey
PidFile $tmp/sshd.pid
AuthorizedKeysFile $tmp/authorized_keys
PasswordAuthentication no
PermitRootLogin yes
PermitUserEnvironment yes
CFG
  /usr/sbin/sshd -f "$tmp/sshd_config"
  trap 'kill $(cat "$tmp/sshd.pid") >/dev/null 2>&1 || true; rm -rf "$tmp"' RETURN
  local ssh="ssh -p $port -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i $clientkey"
  echo "$ssh" "${tmp}"
}

setup_daemon() {
  local server_bin="$1"
  local tmp
  tmp="$(mktemp -d)"
  local port
  port=$(shuf -i 40000-49999 -n 1)
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
  trap 'kill $pid >/dev/null 2>&1 || true; rm -rf "$tmp"' RETURN
  echo "$port" "$tmp/root"
}

make_source() {
  local dir="$1"
  mkdir -p "$dir"
  echo "data" > "$dir/file.txt"
  if command -v setfattr >/dev/null 2>&1; then
    setfattr -n user.test -v value "$dir/file.txt" || true
  fi
  if command -v setfacl >/dev/null 2>&1; then
    setfacl -m u:root:rwx "$dir/file.txt" || true
  fi
}

snapshot_tree() {
  local dir="$1" out="$2" tmp
  tmp="$(mktemp)"
  (
    cd "$dir"
    find . -print0 | sort -z | while IFS= read -r -d '' f; do
      if [[ -f "$f" ]]; then
        sha1sum "$f" || true
      fi
      stat --printf 'meta %n %f %u %g %a %s\n' "$f" || true
      getfacl -p "$f" 2>/dev/null || true
      getfattr -d -m- "$f" 2>/dev/null || true
      echo
    done
  ) > "$out"
  rm -f "$tmp"
}

compare_trees() {
  local a="$1" b="$2" tmp_a tmp_b
  tmp_a="$(mktemp)"
  tmp_b="$(mktemp)"
  snapshot_tree "$a" "$tmp_a"
  snapshot_tree "$b" "$tmp_b"
  diff -u "$tmp_a" "$tmp_b"
  rm -f "$tmp_a" "$tmp_b"
}

# Ensure rsync versions are available
for v in "${CLIENT_VERSIONS[@]}" "${SERVER_VERSIONS[@]}"; do
  fetch_rsync "$v" >/dev/null
done

for c in "${CLIENT_VERSIONS[@]}"; do
  client_bin="$ROOT/rsync-$c/rsync"
  for s in "${SERVER_VERSIONS[@]}"; do
    server_bin="$ROOT/rsync-$s/rsync"
    for t in "${TRANSPORTS[@]}"; do
      echo "Testing client $c server $s via $t" >&2
      tmp="$(mktemp -d)"
      src="$tmp/src"
      dst="$tmp/dst"
      make_source "$src"
      case "$t" in
        ssh)
          read -r ssh tmpdir < <(setup_ssh "$server_bin")
          "$client_bin" "${COMMON_FLAGS[@]}" -e "$ssh" "$src/" "root@localhost:$tmpdir" >/dev/null
          gold="$GOLDEN/${c}_${s}_ssh"
          if [[ "${UPDATE:-0}" == "1" ]]; then
            rm -rf "$gold"
            mv "$tmpdir" "$gold"
          else
            mkdir -p "$dst"
            mv "$tmpdir" "$dst"
            compare_trees "$gold" "$dst"
          fi
          ;;
        rsync)
          read -r port rootdir < <(setup_daemon "$server_bin")
          "$client_bin" "${COMMON_FLAGS[@]}" "$src/" "rsync://localhost:$port/mod" >/dev/null
          gold="$GOLDEN/${c}_${s}_rsync"
          if [[ "${UPDATE:-0}" == "1" ]]; then
            rm -rf "$gold"
            mv "$rootdir" "$gold"
          else
            mkdir -p "$dst"
            mv "$rootdir" "$dst"
            compare_trees "$gold" "$dst"
          fi
          ;;
      esac
      rm -rf "$tmp"
    done
  done
done
