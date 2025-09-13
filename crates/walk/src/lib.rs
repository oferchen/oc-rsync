// crates/walk/src/lib.rs
#![doc = include_str!("../../../docs/crates/walk/lib.md")]
#![forbid(unsafe_code)]

mod fs;
mod ignore;
mod iterator;

pub use iterator::walk;
