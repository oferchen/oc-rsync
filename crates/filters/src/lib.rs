// crates/filters/src/lib.rs
//! Filtering engine for include/exclude rules.
#![deny(unsafe_op_in_unsafe_fn, rust_2018_idioms)]
#![deny(warnings)]
#![warn(missing_docs)]

pub mod matcher;
pub mod parser;
pub mod perdir;
pub mod rule;
pub mod stats;

pub use matcher::{MatchResult, Matcher};
pub use parser::*;
pub use perdir::PerDir;
pub use rule::{Rule, RuleData, RuleFlags};
pub use stats::FilterStats;
