// src/lib.rs

#![doc = include_str!("../docs/crates/oc-rsync/lib.md")]
#![deny(unsafe_code, rust_2018_idioms, missing_debug_implementations)]

pub mod config;
pub mod run;

pub use config::{SyncConfig, SyncConfigBuilder};
pub use meta;
pub use run::{synchronize, synchronize_with_config};
