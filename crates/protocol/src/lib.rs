// crates/protocol/src/lib.rs

pub mod frames;
pub mod handshake;
pub mod types;
pub mod versions;

pub mod demux;
pub mod mux;
pub mod server;

pub use demux::Demux;
pub use mux::{ChannelError, Mux};
pub use server::Server;

pub use frames::{Frame, FrameCodec, FrameHeader};
pub use handshake::{VersionError, negotiate_caps, negotiate_version};
pub use types::{CharsetConv, ExitCode, Message, Msg, Tag, UnknownExit, UnknownMsg, UnknownTag};
pub use versions::{
    CAP_ACLS, CAP_CODECS, CAP_XATTRS, CAP_ZSTD, LATEST_VERSION, MIN_VERSION, SUPPORTED_CAPS,
    SUPPORTED_PROTOCOLS, V30, V31, V32,
};
