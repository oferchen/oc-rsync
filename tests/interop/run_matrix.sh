#!/usr/bin/env bash
set -euo pipefail

# Replay a matrix of recorded transfers using pre-generated goldens under
# tests/interop/golden. Each directory is named
#   <client>_<server>_<transport>[_scenario]
# and captures stdout, stderr, exit status and a tarball of the destination.
# The script no longer invokes the system `rsync`; both client and server are
# the locally built `oc-rsync` binary. When UPDATE=1 is set the goldens are
# regenerated using `oc-rsync`.

ROOT="$(git rev-parse --show-toplevel)"
GOLDEN="$ROOT/tests/interop/golden"
CLIENT_VERSIONS=(${CLIENT_VERSIONS:-"oc-rsync upstream"})
SERVER_VERSIONS=(${SERVER_VERSIONS:-"oc-rsync upstream"})
TRANSPORTS=(${TRANSPORTS:-"ssh rsync"})

if [[ -n "${SCENARIOS:-}" ]]; then
  SCENARIOS_STR="$SCENARIOS"
  IFS=' ' read -r -a SCENARIOS <<< "$SCENARIOS_STR"
else
  # Recorded scenarios. Each entry is "name extra_flags" where extra_flags are
  # optional.
  SCENARIOS=(
    "base"
    "delete --delete"
  )
fi

if [[ "${LIST_SCENARIOS:-0}" == "1" ]]; then
  for entry in "${SCENARIOS[@]}"; do
    IFS=' ' read -r name _ <<< "$entry"
    echo "$name"
  done
  exit 0
fi

# Options used for all transfers. -a implies -rtgoD, we add -A and -X for ACLs
# and xattrs.
COMMON_FLAGS=(${COMMON_FLAGS:-"--archive" "--acls" "--xattrs"})

OC_RSYNC="$ROOT/target/debug/oc-rsync"
if [[ ! -x "$OC_RSYNC" ]]; then
  echo "Building oc-rsync" >&2
  cargo build --quiet --bin oc-rsync --features="acl xattr"
fi

mkdir -p "$GOLDEN"

# Remove protocol banners or other non-deterministic lines from command
# output so that goldens remain stable across runs.
sanitize_banners() {
  sed '/^@RSYNCD:/d'
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
  for _ in {1..50}; do
    [[ -s "$tmp/sshd.pid" ]] && break
    sleep 0.1
  done
  local pid=$(cat "$tmp/sshd.pid")
  local ssh="ssh -p $port -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i $clientkey"
  printf '%s\t%s\t%s\n' "$ssh" "$tmp" "$pid"
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
  printf '%s\t%s\t%s\n' "$port" "$tmp/root" "$pid"
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
  printf '%s\t%s\t%s\n' "$port" "$tmp" "$pid"
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

snapshot_tar() {
  local dir="$1" out="$2"
  tar --sort=name --mtime=@0 --numeric-owner --owner=0 --group=0 \
    --xattrs --acls -cf "$out" -C "$dir" .
}

compare_tree() {
  local gold_tar="$1" dir="$2" tmp
  tmp="$(mktemp)"
  snapshot_tar "$dir" "$tmp"
  if ! cmp -s "$gold_tar" "$tmp"; then
    diff -u <(tar -tvf "$gold_tar") <(tar -tvf "$tmp")
    rm -f "$tmp"
    return 1
  fi
  rm -f "$tmp"
}

for c in "${CLIENT_VERSIONS[@]}"; do
  client_bin="$OC_RSYNC"
  for s in "${SERVER_VERSIONS[@]}"; do
    server_bin="$OC_RSYNC"
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
        make_source "$src"
        case "$t" in
          ssh)
            IFS=$'\t' read -r ssh tmpdir sshpid < <(setup_ssh "$server_bin")
            if [[ "$name" == delete* ]]; then
              touch "$tmpdir/stale.txt"
            fi
            result="$tmpdir"
            if [[ "$name" == drop_connection ]]; then
              dd if=/dev/zero of="$src/big.bin" bs=1M count=20 >/dev/null 2>&1
              timeout 1 "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" --rsync-path "$server_bin" -e "$ssh" "$src/" "root@localhost:$tmpdir" >/dev/null || true
            elif [[ "$name" == resume* ]]; then
              dd if=/dev/zero of="$src/big.bin" bs=1M count=20 >/dev/null 2>&1
              timeout 1 "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" --rsync-path "$server_bin" -e "$ssh" "$src/" "root@localhost:$tmpdir" >/dev/null || true
              "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" --rsync-path "$server_bin" -e "$ssh" "$src/" "root@localhost:$tmpdir" >/dev/null
            elif [[ "$name" == vanished ]]; then
              (sleep 0.1 && rm -f "$src/file.txt") &
              "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" --rsync-path "$server_bin" -e "$ssh" "$src/" "root@localhost:$tmpdir" >/dev/null || true
            elif [[ "$name" == rsh ]]; then
              "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" --rsync-path "$server_bin" --rsh "$ssh" "$src/" "root@localhost:$tmpdir" >/dev/null
            elif [[ "$name" == remote_remote ]]; then
              mkdir -p "$tmpdir/src" "$tmpdir/dst"
              cp -a "$src/." "$tmpdir/src"
              set +e
              "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" --rsync-path "$server_bin" --rsh "$ssh" "root@localhost:$tmpdir/src/" "root@localhost:$tmpdir/dst" >"$tmp/stdout" 2>"$tmp/stderr"
              status=$?
              set -e
              result="$tmpdir/dst"
            else
              set +e
              "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" --rsync-path "$server_bin" -e "$ssh" "$src/" "root@localhost:$tmpdir" >"$tmp/stdout" 2>"$tmp/stderr"
              status=$?
              set -e
            fi
            base="${c}_${s}_ssh"
            gold="$GOLDEN/$base"
            if [[ "$name" != "base" ]]; then
              gold+="_${name}"
            fi
            gold_tar="$gold/tree.tar"
            if [[ "${UPDATE:-0}" == "1" ]]; then
              mkdir -p "$gold"
              sanitize_banners <"$tmp/stdout" >"$gold/stdout"
              sanitize_banners <"$tmp/stderr" >"$gold/stderr"
              echo "$status" >"$gold/exit"
              snapshot_tar "$result" "$gold_tar"
            else
              sanitize_banners <"$tmp/stdout" >"$tmp/stdout.san"
              sanitize_banners <"$tmp/stderr" >"$tmp/stderr.san"
              diff -u "$gold/stdout" "$tmp/stdout.san"
              diff -u "$gold/stderr" "$tmp/stderr.san"
              if [[ "$status" != "$(cat "$gold/exit")" ]]; then
                echo "Exit status mismatch" >&2
                exit 1
              fi
              compare_tree "$gold_tar" "$result"
            fi
            kill "$sshpid" >/dev/null 2>&1 || true
            rm -rf "$tmpdir"
            ;;
          rsync)
            result=""
            if [[ "$name" == remote_remote ]]; then
              IFS=$'\t' read -r port tmpdir daemonpid < <(setup_daemon_rr "$server_bin")
              cp -a "$src/." "$tmpdir/src"
              set +e
              "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" "rsync://localhost:$port/src/" "rsync://localhost:$port/dst" >"$tmp/stdout" 2>"$tmp/stderr"
              status=$?
              set -e
              result="$tmpdir/dst"
              cleanup_dir="$tmpdir"
            else
              IFS=$'\t' read -r port rootdir daemonpid < <(setup_daemon "$server_bin")
              if [[ "$name" == delete* ]]; then
                touch "$rootdir/stale.txt"
              fi
              if [[ "$name" == drop_connection ]]; then
                dd if=/dev/zero of="$src/big.bin" bs=1M count=20 >/dev/null 2>&1
                timeout 1 "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >/dev/null || true
              elif [[ "$name" == resume* ]]; then
                dd if=/dev/zero of="$src/big.bin" bs=1M count=20 >/dev/null 2>&1
                timeout 1 "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >/dev/null || true
                set +e
                "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >"$tmp/stdout" 2>"$tmp/stderr"
                status=$?
                set -e
              elif [[ "$name" == vanished ]]; then
                (sleep 0.1 && rm -f "$src/file.txt") &
                set +e
                "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >"$tmp/stdout" 2>"$tmp/stderr"
                status=$?
                set -e
              else
                set +e
                "$client_bin" "${COMMON_FLAGS[@]}" "${extra[@]}" "$src/" "rsync://localhost:$port/mod" >"$tmp/stdout" 2>"$tmp/stderr"
                status=$?
                set -e
              fi
              result="$rootdir"
              cleanup_dir="$(dirname "$rootdir")"
            fi
            base="${c}_${s}_rsync"
            gold="$GOLDEN/$base"
            if [[ "$name" != "base" ]]; then
              gold+="_${name}"
            fi
            gold_tar="$gold/tree.tar"
            if [[ "${UPDATE:-0}" == "1" ]]; then
              mkdir -p "$gold"
              sanitize_banners <"$tmp/stdout" >"$gold/stdout"
              sanitize_banners <"$tmp/stderr" >"$gold/stderr"
              echo "$status" >"$gold/exit"
              snapshot_tar "$result" "$gold_tar"
            else
              sanitize_banners <"$tmp/stdout" >"$tmp/stdout.san"
              sanitize_banners <"$tmp/stderr" >"$tmp/stderr.san"
              diff -u "$gold/stdout" "$tmp/stdout.san"
              diff -u "$gold/stderr" "$tmp/stderr.san"
              if [[ "$status" != "$(cat "$gold/exit")" ]]; then
                echo "Exit status mismatch" >&2
                exit 1
              fi
              compare_tree "$gold_tar" "$result"
            fi
            kill "$daemonpid" >/dev/null 2>&1 || true
            rm -rf "$cleanup_dir"
            ;;
        esac
        rm -rf "$tmp"
      done
    done
  done
done
