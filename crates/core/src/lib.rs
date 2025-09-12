// crates/core/src/lib.rs
#![doc = include_str!("../../../docs/crates/core/overview.md")]

pub mod fs {
    pub use meta::*;
}

pub mod filter {
    pub use filters::*;
}

pub mod transfer {
    pub use engine::*;
}

pub mod hardlink {
    pub use meta::{HardLinks, hard_link_id};
}

pub mod message {
    pub use protocol::types::*;
}

pub mod config {
    pub use engine::{DeleteMode, SyncOptions};
}
