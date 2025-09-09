// crates/protocol/src/versions.rs
pub const V32: u32 = 32;
pub const V31: u32 = 31;
pub const V30: u32 = 30;

pub const SUPPORTED_PROTOCOLS: &[u32] = &[V32, V31, V30];
pub const LATEST_VERSION: u32 = V32;
pub const MIN_VERSION: u32 = V30;

pub const CAP_CODECS: u32 = 1 << 0;
pub const CAP_ZSTD: u32 = 1 << 1;
pub const CAP_ACLS: u32 = 1 << 2;
pub const CAP_XATTRS: u32 = 1 << 3;

pub const SUPPORTED_CAPS: u32 = CAP_CODECS | CAP_ZSTD | CAP_ACLS | CAP_XATTRS;
