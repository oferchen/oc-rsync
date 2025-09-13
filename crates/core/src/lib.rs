// crates/core/src/lib.rs
#![doc = include_str!("../../../docs/crates/core/overview.md")]
#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, warnings)]

pub mod fs {
    pub use meta::*;
}

pub mod metadata {
    pub use meta::{META_OPTS, MetaOpts};
}

pub mod filter {
    pub use filters::*;
}

pub mod hardlink {
    pub use meta::{HardLinks, hard_link_id};
}

pub mod config {
    pub use engine::{DeleteMode, IdMapper, SyncOptions};
}

pub mod transfer {
    pub use engine::{EngineError, Result, Stats, StrongHash, pipe_sessions, sync};
}

pub mod message {
    pub use protocol::types::*;
    pub use protocol::{CAP_ACLS, CAP_CODECS, CAP_XATTRS, SUPPORTED_PROTOCOLS, negotiate_version};
}

pub mod compress {
    pub use compress::*;
}

pub mod checksums {
    pub use checksums::*;
}

pub use engine::{PathSpec, RemoteSpec, is_remote_spec, parse_remote_spec};
