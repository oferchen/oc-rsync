// crates/core/src/lib.rs
#![doc = include_str!("../../../docs/crates/core/overview.md")]

pub mod fs {
    pub use meta::*;
}

pub mod filter {
    pub use filters::*;
}

pub mod hardlink {
    pub use meta::{HardLinks, hard_link_id};
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
