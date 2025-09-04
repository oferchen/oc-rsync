#!/usr/bin/env bash
set -euo pipefail

cargo nextest run --no-fail-fast --test daemon -- --exact host_allow_supports_cidr
cargo nextest run --no-fail-fast --test daemon -- --exact daemon_refuses_configured_option
cargo nextest run --no-fail-fast --test daemon -- --exact daemon_refuses_numeric_ids_option
cargo nextest run --no-fail-fast --test daemon -- --exact daemon_refuses_no_numeric_ids_option
cargo nextest run --no-fail-fast --test daemon -- --exact chroot_drops_privileges
cargo nextest run --no-fail-fast --test daemon -- --exact chroot_requires_root
cargo nextest run --no-fail-fast --test daemon -- --exact chroot_and_drop_privileges_rejects_missing_dir
cargo nextest run --no-fail-fast --test daemon -- --exact drop_privileges_requires_root
