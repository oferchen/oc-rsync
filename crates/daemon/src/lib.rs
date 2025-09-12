// crates/daemon/src/lib.rs
#![doc = include_str!("../../../docs/crates/daemon/lib.md")]
#![allow(clippy::collapsible_if)]

pub mod auth;
pub mod config;
pub mod service;

pub use auth::{authenticate, authenticate_token, parse_auth_token};
pub use config::{
    DaemonArgs, DaemonConfig, Module, ModuleBuilder, load_config, parse_config, parse_config_file,
    parse_daemon_args, parse_module,
};
pub use service::{
    Handler, PrivilegeContext, chroot_and_drop_privileges, drop_privileges, handle_connection,
    host_allowed, init_logging, run_daemon, serve_module,
};

pub use oc_rsync_core::metadata::{META_OPTS, MetaOpts};
