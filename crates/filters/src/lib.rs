// crates/filters/src/lib.rs
//! Root of the filters crate.
//!
//! [`Matcher`] now honors the `--no-implied-dirs` flag via
//! [`Matcher::with_no_implied_dirs`], ensuring implied directory semantics
//! match upstream rsync.
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
