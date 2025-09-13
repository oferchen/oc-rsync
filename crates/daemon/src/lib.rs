// crates/daemon/src/lib.rs
#![deny(unsafe_code)]
#![deny(rust_2018_idioms, warnings)]
#![doc = include_str!("../../../docs/crates/daemon/lib.md")]
#![allow(clippy::collapsible_if)]

pub mod auth;
pub mod config;
mod os;
pub mod service;

pub use auth::{authenticate, authenticate_token, parse_auth_token};
pub use config::{
    load_config, parse_config, parse_config_file, parse_daemon_args, parse_module, DaemonArgs,
    DaemonConfig, Module, ModuleBuilder,
};
pub use service::{
    chroot_and_drop_privileges, drop_privileges, handle_connection, host_allowed, init_logging,
    run_daemon, serve_module, Handler, PrivilegeContext,
};

pub use oc_rsync_core::metadata::{MetaOpts, META_OPTS};
