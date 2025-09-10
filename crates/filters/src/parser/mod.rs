// crates/filters/src/parser/mod.rs
#![allow(clippy::collapsible_if)]

pub const MAX_PARSE_DEPTH: usize = 64;
pub const MAX_BRACE_EXPANSIONS: usize = 10_000;

pub(crate) mod braces;
pub(crate) mod cvs;
pub(crate) mod error;
pub(crate) mod glob;
pub(crate) mod list;
pub(crate) mod parse;
pub(crate) mod util;

pub(crate) use braces::*;
pub(crate) use glob::*;
pub(crate) use util::*;

pub use cvs::default_cvs_rules;
pub use error::ParseError;
pub use list::{
    parse_file, parse_from_bytes, parse_list_file, parse_rule_list_file, parse_rule_list_from_bytes,
};
pub use parse::{parse, parse_with_options};
