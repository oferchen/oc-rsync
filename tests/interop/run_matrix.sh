#!/usr/bin/env bash
set -euo pipefail

# Run a matrix of rsync client/server combinations over SSH and rsync://
# connections. Golden directory trees are stored under tests/interop/golden
# using the layout:
#   golden/<client>_<server>_<transport>
# When UPDATE=1 is set the goldens are regenerated.

ROOT="$(git rev-parse --show-toplevel)"
GOLDEN="$ROOT/tests/interop/golden"
CLIENT_VERSIONS=("3.1.3" "3.2.7" "3.3.0" "3.4.0" "oc-rsync")
SERVER_VERSIONS=("3.1.3" "3.2.7" "3.3.0" "3.4.0" "oc-rsync")
TRANSPORTS=("ssh" "rsync")

# Additional scenarios to exercise. Each entry is "name extra_flags" where
# extra_flags are optional.
SCENARIOS=(
  "base"
  "delete --delete"
  "delete_before --delete-before"
  "delete_during --delete-during"
  "delete_after --delete-after"
  "compress --compress"
  "rsh"
  "drop_connection"
  "vanished"
  "remote_remote"
  "append --append"
  "append_verify --append-verify"
  "resume --partial"
  "progress --progress"
)

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

setup_daemon_rr() {
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
[src]
  path = $tmp/src
  read only = false
[dst]
  path = $tmp/dst
  read only = false
CFG
  mkdir -p "$tmp/src" "$tmp/dst"
  "$server_bin" --daemon --no-detach --port "$port" --config "$tmp/rsyncd.conf" &
  local pid=$!
  trap 'kill $pid >/dev/null 2>&1 || true; rm -rf "$tmp"' RETURN
  echo "$port" "$tmp"
}

make_source() {
  local dir="$1"
  mkdir -p "$dir"
  echo "data" > "$dir/file.txt"
  ln "$dir/file.txt" "$dir/hardlink.txt"
  ln -s file.txt "$dir/symlink.txt"
  mknod "$dir/char.dev" c 1 3
  dd if=/dev/zero of="$dir/sparse.bin" bs=1 count=0 seek=1M >/dev/null 2>&1
  if command -v setfattr >/dev/null 2>&1; then
    setfattr -n user.test -v value "$dir/file.txt" || true
    setfattr -n user.sparse -v value "$dir/sparse.bin" || true
    setfattr -h -n user.link -v value "$dir/symlink.txt" || true
  fi
  if command -v setfacl >/dev/null 2>&1; then
    setfacl -m u:root:rwx "$dir/file.txt" || true
    setfacl -m u:root:rwx "$dir/sparse.bin" || true
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
      if [[ -L "$f" ]]; then
        printf 'link %s -> %s\n' "$f" "$(readlink "$f")" || true
      fi
      stat --printf 'meta %n %f %u %g %a %s %b %i %t:%T\n' "$f" || true
      getfacl -h -p "$f" 2>/dev/null || true
      getfattr -h -d -m- "$f" 2>/dev/null || true
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
  if [[ "$v" == "oc-rsync" ]]; then
    if [[ ! -x "$ROOT/target/debug/oc-rsync" ]]; then
      echo "Building oc-rsync" >&2
      cargo build --quiet -p oc-rsync-bin --bin oc-rsync
    fi
  else
    fetch_rsync "$v" >/dev/null
  fi
done

for c in "${CLIENT_VERSIONS[@]}"; do
  if [[ "$c" == "oc-rsync" ]]; then
    client_bin="$ROOT/target/debug/oc-rsync"
  else
    client_bin="$ROOT/rsync-$c/rsync"
  fi
  for s in "${SERVER_VERSIONS[@]}"; do
    if [[ "$s" == "oc-rsync" ]]; then
      server_bin="$ROOT/target/debug/oc-rsync"
    else
      server_bin="$ROOT/rsync-$s/rsync"
    fi
    for t in "${TRANSPORTS[@]}"; do
      for scenario in "${SCENARIOS[@]}"; do
        IFS=' ' read -r name flag <<< "$scenario"
        extra=()
        if [[ -n "${flag:-}" ]]; then
          extra=($flag)
        fi
        if [[ "$name" == "rsh" && "$t" != "ssh" ]]; then
          continue
        fi
        echo "Testing client $c server $s via $t scenario $name" >&2
        tmp="$(mktemp -d)"
        src="$tmp/src"
        dst="$tmp/dst"
        make_source "$src"
        case "$t" in
          ssh)
            read -r ssh tmpdir < <(setup_ssh "$server_bin")
            if [[ "$name" == delete* ]]; then
              touch "$tmpdir/stale.txt"
            fi
            result="$tmpdir"
            if [[ "$name" == drop_connection ]]; then
              dd if=/dev/zero of="$src/big.bin" bs=1M count=20 >/dev/null 2>&1
              timeout 1 "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" -e "$ssh" "$src/" "root@localhost:$tmpdir" >/dev/null || true
            elif [[ "$name" == resume ]]; then
              dd if=/dev/zero of="$src/big.bin" bs=1M count=20 >/dev/null 2>&1
              timeout 1 "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" -e "$ssh" "$src/" "root@localhost:$tmpdir" >/dev/null || true
              "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" -e "$ssh" "$src/" "root@localhost:$tmpdir" >/dev/null
            elif [[ "$name" == vanished ]]; then
              (sleep 0.1 && rm -f "$src/file.txt") &
              "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" -e "$ssh" "$src/" "root@localhost:$tmpdir" >/dev/null || true
            elif [[ "$name" == rsh ]]; then
              "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" --rsh "$ssh" "$src/" "root@localhost:$tmpdir" >/dev/null
            elif [[ "$name" == remote_remote ]]; then
              mkdir -p "$tmpdir/src" "$tmpdir/dst"
              cp -a "$src/." "$tmpdir/src"
              "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" --rsh "$ssh" "root@localhost:$tmpdir/src/" "root@localhost:$tmpdir/dst" >/dev/null
              result="$tmpdir/dst"
            else
              "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" -e "$ssh" "$src/" "root@localhost:$tmpdir" >/dev/null
            fi
            base="${c}_${s}_ssh"
            gold="$GOLDEN/$base"
            if [[ "$name" != "base" ]]; then
              gold+="_${name}"
            fi
            if [[ "${UPDATE:-0}" == "1" ]]; then
              rm -rf "$gold"
              mv "$result" "$gold"
            else
              mkdir -p "$dst"
              mv "$result" "$dst"
              compare_trees "$gold" "$dst"
            fi
            ;;
          rsync)
            result=""
            if [[ "$name" == remote_remote ]]; then
              read -r port tmpdir < <(setup_daemon_rr "$server_bin")
              cp -a "$src/." "$tmpdir/src"
              "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" "rsync://localhost:$port/src/" "rsync://localhost:$port/dst" >/dev/null
              result="$tmpdir/dst"
            else
              read -r port rootdir < <(setup_daemon "$server_bin")
              if [[ "$name" == delete* ]]; then
                touch "$rootdir/stale.txt"
              fi
              if [[ "$name" == drop_connection ]]; then
                dd if=/dev/zero of="$src/big.bin" bs=1M count=20 >/dev/null 2>&1
                timeout 1 "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >/dev/null || true
              elif [[ "$name" == resume ]]; then
                dd if=/dev/zero of="$src/big.bin" bs=1M count=20 >/dev/null 2>&1
                timeout 1 "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >/dev/null || true
                "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >/dev/null
              elif [[ "$name" == vanished ]]; then
                (sleep 0.1 && rm -f "$src/file.txt") &
                "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >/dev/null || true
              else
                "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >/dev/null
              fi
              result="$rootdir"
            fi
            base="${c}_${s}_rsync"
            gold="$GOLDEN/$base"
            if [[ "$name" != "base" ]]; then
              gold+="_${name}"
            fi
            if [[ "${UPDATE:-0}" == "1" ]]; then
              rm -rf "$gold"
              mv "$result" "$gold"
            else
              mkdir -p "$dst"
              mv "$result" "$dst"
              compare_trees "$gold" "$dst"
            fi
            ;;
        esac
        rm -rf "$tmp"
      done
    done
  done
done
