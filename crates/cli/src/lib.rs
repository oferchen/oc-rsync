// crates/cli/src/lib.rs

#![allow(clippy::collapsible_if)]
#![deny(unsafe_op_in_unsafe_fn, rust_2018_idioms, warnings)]
#![doc = "Command-line interface for oc-rsync."]

mod argparse;
pub mod branding;
mod client;
pub mod daemon;
mod exec;
mod formatter;
mod print;
mod probe;
mod session;
mod transport_factory;
mod utils;
mod validate;
pub mod version;

pub mod options {
    pub use crate::argparse::*;
}

pub use client::run;
pub use daemon::spawn_daemon_session;
pub use engine::EngineError;
pub use formatter::{ARG_ORDER, dump_help_body, render_help};
pub use options::cli_command;
pub use print::handle_clap_error;
pub use utils::{
    PathSpec, RemoteSpec, parse_iconv, parse_logging_flags, parse_remote_spec, parse_rsh,
    print_version_if_requested,
};
pub use validate::{
    ClientOptsBuilder, ProbeOptsBuilder, exit_code_from_engine_error, exit_code_from_error_kind,
    validate_paths,
};
