// crates/protocol/src/lib.rs
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::convert::TryFrom;
use std::fmt;
use std::io::{self, Read, Write};

pub mod demux;
pub mod mux;
pub mod server;
pub use demux::Demux;
pub use mux::Mux;
pub use server::Server;

pub const V32: u32 = 32;
pub const V31: u32 = 31;
pub const LATEST_VERSION: u32 = V32;
pub const MIN_VERSION: u32 = V31;

pub const CAP_CODECS: u32 = 1 << 0;
pub const CAP_BLAKE3: u32 = 1 << 1;
pub const CAP_ZSTD: u32 = 1 << 2;
pub const CAP_LZ4: u32 = 1 << 3;
pub const SUPPORTED_CAPS: u32 = CAP_CODECS | CAP_BLAKE3 | CAP_ZSTD | CAP_LZ4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionError(pub u32);

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported version {}", self.0)
    }
}

impl std::error::Error for VersionError {}

impl From<VersionError> for io::Error {
    fn from(e: VersionError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, e)
    }
}

pub fn negotiate_version(local: u32, peer: u32) -> Result<u32, VersionError> {
    if local >= V32 && peer >= V32 {
        Ok(V32)
    } else if local >= V31 && peer >= V31 {
        Ok(V31)
    } else {
        Err(VersionError(local.min(peer)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Tag {
    Message = 0,
    KeepAlive = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownTag(pub u8);

impl fmt::Display for UnknownTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown tag {}", self.0)
    }
}

impl std::error::Error for UnknownTag {}

impl From<UnknownTag> for io::Error {
    fn from(e: UnknownTag) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, e)
    }
}

impl TryFrom<u8> for Tag {
    type Error = UnknownTag;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Tag::Message),
            1 => Ok(Tag::KeepAlive),
            other => Err(UnknownTag(other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Msg {
    Version = 0,
    Data = 1,
    Done = 2,
    KeepAlive = 3,
    FileListEntry = 4,
    Attributes = 5,
    Error = 6,
    Progress = 7,
    Codecs = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownMsg(pub u8);

impl fmt::Display for UnknownMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown message {}", self.0)
    }
}

impl std::error::Error for UnknownMsg {}

impl From<UnknownMsg> for io::Error {
    fn from(e: UnknownMsg) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, e)
    }
}

impl TryFrom<u8> for Msg {
    type Error = UnknownMsg;

    fn try_from(v: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match v {
            0 => Ok(Msg::Version),
            1 => Ok(Msg::Data),
            2 => Ok(Msg::Done),
            3 => Ok(Msg::KeepAlive),
            4 => Ok(Msg::FileListEntry),
            5 => Ok(Msg::Attributes),
            6 => Ok(Msg::Error),
            7 => Ok(Msg::Progress),
            8 => Ok(Msg::Codecs),
            other => Err(UnknownMsg(other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    Ok = 0,
    SyntaxOrUsage = 1,
    Protocol = 2,
    FileSelect = 3,
    Unsupported = 4,
    StartClient = 5,
    SocketIo = 10,
    FileIo = 11,
    StreamIo = 12,
    MessageIo = 13,
    Ipc = 14,
    Crashed = 15,
    Terminated = 16,
    Signal1 = 19,
    Signal = 20,
    WaitChild = 21,
    Malloc = 22,
    Partial = 23,
    Vanished = 24,
    DelLimit = 25,
    Timeout = 30,
    ConnTimeout = 35,
    CmdFailed = 124,
    CmdKilled = 125,
    CmdRun = 126,
    CmdNotFound = 127,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownExit(pub u8);

impl fmt::Display for UnknownExit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown exit code {}", self.0)
    }
}

impl std::error::Error for UnknownExit {}

impl TryFrom<u8> for ExitCode {
    type Error = UnknownExit;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(ExitCode::Ok),
            1 => Ok(ExitCode::SyntaxOrUsage),
            2 => Ok(ExitCode::Protocol),
            3 => Ok(ExitCode::FileSelect),
            4 => Ok(ExitCode::Unsupported),
            5 => Ok(ExitCode::StartClient),
            10 => Ok(ExitCode::SocketIo),
            11 => Ok(ExitCode::FileIo),
            12 => Ok(ExitCode::StreamIo),
            13 => Ok(ExitCode::MessageIo),
            14 => Ok(ExitCode::Ipc),
            15 => Ok(ExitCode::Crashed),
            16 => Ok(ExitCode::Terminated),
            19 => Ok(ExitCode::Signal1),
            20 => Ok(ExitCode::Signal),
            21 => Ok(ExitCode::WaitChild),
            22 => Ok(ExitCode::Malloc),
            23 => Ok(ExitCode::Partial),
            24 => Ok(ExitCode::Vanished),
            25 => Ok(ExitCode::DelLimit),
            30 => Ok(ExitCode::Timeout),
            35 => Ok(ExitCode::ConnTimeout),
            124 => Ok(ExitCode::CmdFailed),
            125 => Ok(ExitCode::CmdKilled),
            126 => Ok(ExitCode::CmdRun),
            127 => Ok(ExitCode::CmdNotFound),
            other => Err(UnknownExit(other)),
        }
    }
}

impl From<ExitCode> for u8 {
    fn from(e: ExitCode) -> Self {
        e as u8
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    pub channel: u16,
    pub tag: Tag,
    pub msg: Msg,
    pub len: u32,
}

impl FrameHeader {
    fn encode<W: Write>(&self, mut w: W) -> io::Result<()> {
        w.write_u16::<BigEndian>(self.channel)?;
        w.write_u8(self.tag as u8)?;
        w.write_u8(self.msg as u8)?;
        w.write_u32::<BigEndian>(self.len)?;
        Ok(())
    }

    fn decode<R: Read>(mut r: R) -> io::Result<Self> {
        let channel = r.read_u16::<BigEndian>()?;
        let tag_byte = r.read_u8()?;
        let tag = Tag::try_from(tag_byte).map_err(io::Error::from)?;
        let msg_byte = r.read_u8()?;
        let msg = Msg::try_from(msg_byte).map_err(io::Error::from)?;
        let len = r.read_u32::<BigEndian>()?;
        Ok(FrameHeader {
            channel,
            tag,
            msg,
            len,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub header: FrameHeader,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn encode<W: Write>(&self, mut w: W) -> io::Result<()> {
        self.header.encode(&mut w)?;
        w.write_all(&self.payload)
    }

    pub fn decode<R: Read>(mut r: R) -> io::Result<Self> {
        let header = FrameHeader::decode(&mut r)?;
        let mut payload = vec![0; header.len as usize];
        r.read_exact(&mut payload)?;
        Ok(Frame { header, payload })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Version(u32),
    Data(Vec<u8>),
    Done,
    KeepAlive,
    FileListEntry(Vec<u8>),
    Attributes(Vec<u8>),
    Error(String),
    Progress(u64),
    Codecs(Vec<u8>),
}

impl Message {
    pub fn into_frame(self, channel: u16) -> Frame {
        match self {
            Message::Version(v) => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&v.to_be_bytes());
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Version,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Data(payload) => {
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Data,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Done => {
                let payload = Vec::new();
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Done,
                    len: 0,
                };
                Frame { header, payload }
            }
            Message::KeepAlive => {
                let payload = Vec::new();
                let header = FrameHeader {
                    channel,
                    tag: Tag::KeepAlive,
                    msg: Msg::KeepAlive,
                    len: 0,
                };
                Frame { header, payload }
            }
            Message::FileListEntry(payload) => {
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::FileListEntry,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Attributes(payload) => {
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Attributes,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Error(text) => {
                let payload = text.into_bytes();
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Error,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Progress(v) => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&v.to_be_bytes());
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Progress,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Codecs(payload) => {
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Codecs,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
        }
    }

    pub fn to_frame(&self, channel: u16) -> Frame {
        self.clone().into_frame(channel)
    }

    pub fn from_frame(f: Frame) -> io::Result<Self> {
        if f.header.len as usize != f.payload.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "frame length mismatch",
            ));
        }
        match f.header.tag {
            Tag::KeepAlive => match f.header.msg {
                Msg::KeepAlive => Ok(Message::KeepAlive),
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid keepalive message",
                )),
            },
            Tag::Message => match f.header.msg {
                Msg::Version => {
                    let mut rdr = &f.payload[..];
                    let v = rdr.read_u32::<BigEndian>()?;
                    Ok(Message::Version(v))
                }
                Msg::Data => Ok(Message::Data(f.payload)),
                Msg::Done => Ok(Message::Done),
                Msg::FileListEntry => Ok(Message::FileListEntry(f.payload)),
                Msg::Attributes => Ok(Message::Attributes(f.payload)),
                Msg::Error => {
                    let text = String::from_utf8(f.payload)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    Ok(Message::Error(text))
                }
                Msg::Progress => {
                    if f.payload.len() != 8 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "invalid progress payload",
                        ));
                    }
                    let mut rdr = &f.payload[..];
                    let v = rdr.read_u64::<BigEndian>()?;
                    Ok(Message::Progress(v))
                }
                Msg::Codecs => Ok(Message::Codecs(f.payload)),
                Msg::KeepAlive => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unexpected keepalive message",
                )),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_negotiation() {
        assert_eq!(negotiate_version(V32, V32), Ok(V32));
        assert_eq!(negotiate_version(V32, V31), Ok(V31));
        assert_eq!(negotiate_version(V31, V32), Ok(V31));
        assert_eq!(negotiate_version(V31, V31), Ok(V31));
        assert!(negotiate_version(V32, 30).is_err());
    }

    #[test]
    fn frame_roundtrip() {
        let msg = Message::Data(b"hello".to_vec());
        let frame = msg.to_frame(7);
        let mut buf = Vec::new();
        frame.encode(&mut buf).unwrap();
        let decoded = Frame::decode(&buf[..]).unwrap();
        assert_eq!(decoded, frame);
        let msg2 = Message::from_frame(decoded).unwrap();
        assert_eq!(msg2, msg);

        let msg4 = Message::Data(b"1234".to_vec());
        let frame4 = msg4.to_frame(3);
        let mut buf4 = Vec::new();
        frame4.encode(&mut buf4).unwrap();
        let decoded4 = Frame::decode(&buf4[..]).unwrap();
        assert_eq!(decoded4, frame4);
        let msg4_round = Message::from_frame(decoded4).unwrap();
        assert_eq!(msg4_round, msg4);

        let msgc = Message::Codecs(vec![0, 1]);
        let framec = msgc.to_frame(1);
        let mut bufc = Vec::new();
        framec.encode(&mut bufc).unwrap();
        let decodedc = Frame::decode(&bufc[..]).unwrap();
        assert_eq!(decodedc, framec);
        let msgc_round = Message::from_frame(decodedc).unwrap();
        assert_eq!(msgc_round, msgc);
    }

    #[test]
    fn keepalive_frame() {
        let msg = Message::KeepAlive;
        let frame = msg.into_frame(0);
        assert_eq!(frame.header.tag, Tag::KeepAlive);
        assert_eq!(frame.header.msg, Msg::KeepAlive);
        let mut buf = Vec::new();
        frame.encode(&mut buf).unwrap();
        let decoded = Frame::decode(&buf[..]).unwrap();
        let msg2 = Message::from_frame(decoded).unwrap();
        assert_eq!(msg2, Message::KeepAlive);
    }

    #[test]
    fn too_short_payload_errors() {
        let frame = Frame {
            header: FrameHeader {
                channel: 0,
                tag: Tag::Message,
                msg: Msg::Data,
                len: 10,
            },
            payload: vec![0; 5],
        };
        assert!(Message::from_frame(frame).is_err());
    }

    #[test]
    fn too_long_payload_errors() {
        let frame = Frame {
            header: FrameHeader {
                channel: 0,
                tag: Tag::Message,
                msg: Msg::Data,
                len: 1,
            },
            payload: vec![0; 5],
        };
        assert!(Message::from_frame(frame).is_err());
    }

    #[test]
    fn unknown_tag_errors() {
        let buf = [0u8, 0, 99, 0, 0, 0, 0];
        assert!(Frame::decode(&buf[..]).is_err());
    }

    #[test]
    fn unknown_msg_errors() {
        let buf = [0u8, 0, 0, 99, 0, 0, 0, 0];
        assert!(Frame::decode(&buf[..]).is_err());
    }
}
