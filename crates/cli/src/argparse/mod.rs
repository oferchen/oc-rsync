// crates/cli/src/argparse/mod.rs

pub mod builder;
pub mod flags;
pub mod validation;

pub use crate::daemon::DaemonOpts;
pub use builder::{ClientOptsBuilder, ProbeOptsBuilder, cli_command};
pub use flags::{ClientOpts, OutBuf, ProbeOpts};
pub use validation::{exit_code_from_engine_error, exit_code_from_error_kind, validate_paths};
