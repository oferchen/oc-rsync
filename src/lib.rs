// src/lib.rs

#![doc = include_str!("../docs/crates/oc-rsync/lib.md")]
#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, missing_debug_implementations)]

pub mod config;
pub mod run;

pub use config::{SyncConfig, SyncConfigBuilder};
pub use meta;
use oc_rsync_core as _;
pub use run::{synchronize, synchronize_with_config};
