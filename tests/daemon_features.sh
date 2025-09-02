#!/usr/bin/env bash
set -euo pipefail

cargo test --test daemon -- --exact host_allow_supports_cidr
cargo test --test daemon -- --exact daemon_refuses_configured_option
cargo test --test daemon -- --exact daemon_refuses_numeric_ids_option
cargo test --test daemon -- --exact daemon_refuses_no_numeric_ids_option
