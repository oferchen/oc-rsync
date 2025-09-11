// crates/filters/src/matcher/mod.rs
#![allow(clippy::collapsible_if)]

mod core;
mod delete;
mod evaluate;
mod merge;
mod path;
mod xattr;

pub use core::{MatchResult, Matcher};
