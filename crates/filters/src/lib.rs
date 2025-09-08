// crates/filters/src/lib.rs

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
