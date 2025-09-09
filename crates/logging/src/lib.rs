// crates/logging/src/lib.rs
#![deny(missing_docs, rust_2018_idioms)]
#![doc = "Logging utilities for oc-rsync."]

mod flags;
mod formatter;
mod json_format;
mod sink;
mod subscriber;
mod util;

pub use flags::{
    DebugFlag, InfoFlag, LogFormat, StderrMode, SubscriberConfig, SubscriberConfigBuilder,
};
pub use formatter::RsyncFormatter;
pub use sink::{NopObserver, Observer};
pub use subscriber::{init, subscriber};
pub use util::{
    OutFormatOptions, escape_path, human_bytes, parse_escapes, progress_formatter, rate_formatter,
    render_out_format,
};
