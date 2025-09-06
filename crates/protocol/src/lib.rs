// crates/protocol/src/lib.rs
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use encoding_rs::Encoding;
use filelist::{Decoder as FlistDecoder, Encoder as FlistEncoder, Entry as FlistEntry};
use std::borrow::Cow;
use std::convert::TryFrom;
use std::fmt;
use std::io::{self, Read, Write};

#[derive(Clone)]
pub struct CharsetConv {
    remote: &'static Encoding,
    local: &'static Encoding,
}

impl CharsetConv {
    pub fn encode_remote<'a>(&self, s: &'a str) -> Cow<'a, [u8]> {
        let (res, _, _) = self.remote.encode(s);
        res
    }

    pub fn decode_remote<'a>(&self, b: &'a [u8]) -> Cow<'a, str> {
        let (res, _, _) = self.remote.decode(b);
        res
    }

    pub fn to_remote<'a>(&self, b: &'a [u8]) -> Cow<'a, [u8]> {
        if self.remote == self.local {
            Cow::Borrowed(b)
        } else {
            let (s, _, _) = self.local.decode(b);
            Cow::Owned(self.remote.encode(&s).0.into_owned())
        }
    }

    pub fn to_local<'a>(&self, b: &'a [u8]) -> Cow<'a, [u8]> {
        if self.remote == self.local {
            Cow::Borrowed(b)
        } else {
            let (s, _, _) = self.remote.decode(b);
            Cow::Owned(self.local.encode(&s).0.into_owned())
        }
    }

    pub fn new(remote: &'static Encoding, local: &'static Encoding) -> Self {
        Self { remote, local }
    }
}

pub mod demux;
pub mod mux;
pub mod server;
pub use demux::Demux;
pub use mux::{ChannelError, Mux};
pub use server::Server;

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
    for &v in SUPPORTED_PROTOCOLS {
        if local >= v && peer >= v {
            return Ok(v);
        }
    }
    Err(VersionError(local.min(peer)))
}

pub fn negotiate_caps(local: u32, peer: u32) -> u32 {
    (local & peer) & SUPPORTED_CAPS
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
    Data = 0,
    ErrorXfer = 1,
    Info = 2,
    Error = 3,
    Warning = 4,
    ErrorSocket = 5,
    Log = 6,
    Client = 7,
    ErrorUtf8 = 8,
    Redo = 9,
    Stats = 10,
    IoError = 22,
    IoTimeout = 33,
    Noop = 42,
    ErrorExit = 86,
    Success = 100,
    Deleted = 101,
    NoSend = 102,
    Version = 0xF0,
    Done = 0xF1,
    KeepAlive = 0xF2,
    FileListEntry = 0xF3,
    Attributes = 0xF4,
    Progress = 0xF5,
    Codecs = 0xF6,
    Xattrs = 0xF7,
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
            0 => Ok(Msg::Data),
            1 => Ok(Msg::ErrorXfer),
            2 => Ok(Msg::Info),
            3 => Ok(Msg::Error),
            4 => Ok(Msg::Warning),
            5 => Ok(Msg::ErrorSocket),
            6 => Ok(Msg::Log),
            7 => Ok(Msg::Client),
            8 => Ok(Msg::ErrorUtf8),
            9 => Ok(Msg::Redo),
            10 => Ok(Msg::Stats),
            22 => Ok(Msg::IoError),
            33 => Ok(Msg::IoTimeout),
            42 => Ok(Msg::Noop),
            86 => Ok(Msg::ErrorExit),
            100 => Ok(Msg::Success),
            101 => Ok(Msg::Deleted),
            102 => Ok(Msg::NoSend),
            0xF0 => Ok(Msg::Version),
            0xF1 => Ok(Msg::Done),
            0xF2 => Ok(Msg::KeepAlive),
            0xF3 => Ok(Msg::FileListEntry),
            0xF4 => Ok(Msg::Attributes),
            0xF5 => Ok(Msg::Progress),
            0xF6 => Ok(Msg::Codecs),
            0xF7 => Ok(Msg::Xattrs),
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
    DaemonConfig = 6,
    SocketIo = 10,
    FileIo = 11,
    StreamIo = 12,
    MessageIo = 13,
    Ipc = 14,
    Crashed = 15,
    Terminated = 16,
    Signal1 = 19,
    Signal = 20,
    WaitPid = 21,
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
            6 => Ok(ExitCode::DaemonConfig),
            10 => Ok(ExitCode::SocketIo),
            11 => Ok(ExitCode::FileIo),
            12 => Ok(ExitCode::StreamIo),
            13 => Ok(ExitCode::MessageIo),
            14 => Ok(ExitCode::Ipc),
            15 => Ok(ExitCode::Crashed),
            16 => Ok(ExitCode::Terminated),
            19 => Ok(ExitCode::Signal1),
            20 => Ok(ExitCode::Signal),
            21 => Ok(ExitCode::WaitPid),
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
    Xattrs(Vec<u8>),
    ErrorXfer(String),
    Info(String),
    Error(String),
    Warning(String),
    ErrorSocket(String),
    Log(String),
    Client(String),
    ErrorUtf8(String),
    Progress(u64),
    Codecs(Vec<u8>),
    IoError(u32),
    IoTimeout(u32),
    Noop,
    Redo(u32),
    Stats(Vec<u8>),
    Exit(u8),
    Success(u32),
    Deleted(u32),
    NoSend(u32),
    Other(Msg, Vec<u8>),
}

impl Message {
    pub fn into_frame(self, channel: u16, iconv: Option<&CharsetConv>) -> Frame {
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
            Message::Xattrs(payload) => {
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Xattrs,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::ErrorXfer(text) => Self::encode_text(channel, Msg::ErrorXfer, text, iconv),
            Message::Info(text) => Self::encode_text(channel, Msg::Info, text, iconv),
            Message::Error(text) => Self::encode_text(channel, Msg::Error, text, iconv),
            Message::Warning(text) => Self::encode_text(channel, Msg::Warning, text, iconv),
            Message::ErrorSocket(text) => Self::encode_text(channel, Msg::ErrorSocket, text, iconv),
            Message::Log(text) => Self::encode_text(channel, Msg::Log, text, iconv),
            Message::Client(text) => Self::encode_text(channel, Msg::Client, text, iconv),
            Message::ErrorUtf8(text) => Self::encode_text(channel, Msg::ErrorUtf8, text, iconv),
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
            Message::IoError(code) => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&code.to_be_bytes());
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::IoError,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::IoTimeout(code) => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&code.to_be_bytes());
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::IoTimeout,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Noop => {
                let payload = Vec::new();
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Noop,
                    len: 0,
                };
                Frame { header, payload }
            }
            Message::Redo(idx) => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&idx.to_be_bytes());
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Redo,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Stats(payload) => {
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Stats,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Exit(code) => {
                let payload = vec![code];
                let msg = if code == 0 { Msg::Data } else { Msg::ErrorExit };
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Success(idx) => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&idx.to_be_bytes());
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Success,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Deleted(idx) => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&idx.to_be_bytes());
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Deleted,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::NoSend(idx) => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&idx.to_be_bytes());
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::NoSend,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Other(msg, payload) => {
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
        }
    }

    fn encode_text(channel: u16, msg: Msg, text: String, iconv: Option<&CharsetConv>) -> Frame {
        let payload = if let Some(cv) = iconv {
            cv.encode_remote(&text).into_owned()
        } else {
            text.into_bytes()
        };
        let header = FrameHeader {
            channel,
            tag: Tag::Message,
            msg,
            len: payload.len() as u32,
        };
        Frame { header, payload }
    }

    fn decode_text(payload: Vec<u8>, iconv: Option<&CharsetConv>) -> io::Result<String> {
        if let Some(cv) = iconv {
            Ok(cv.decode_remote(&payload).into_owned())
        } else {
            String::from_utf8(payload).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
    }

    pub fn to_frame(&self, channel: u16, iconv: Option<&CharsetConv>) -> Frame {
        self.clone().into_frame(channel, iconv)
    }

    pub fn from_frame(f: Frame, iconv: Option<&CharsetConv>) -> io::Result<Self> {
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
            Tag::Message =>
            {
                #[allow(unreachable_patterns)]
                match f.header.msg {
                    Msg::Version => {
                        let mut rdr = &f.payload[..];
                        let v = rdr.read_u32::<BigEndian>()?;
                        Ok(Message::Version(v))
                    }
                    Msg::Data => {
                        if f.header.channel == 0 && f.payload.len() == 1 {
                            Ok(Message::Exit(f.payload[0]))
                        } else {
                            Ok(Message::Data(f.payload))
                        }
                    }
                    Msg::Done => Ok(Message::Done),
                    Msg::FileListEntry => Ok(Message::FileListEntry(f.payload)),
                    Msg::Attributes => Ok(Message::Attributes(f.payload)),
                    Msg::Xattrs => Ok(Message::Xattrs(f.payload)),
                    Msg::ErrorXfer => {
                        let text = Self::decode_text(f.payload, iconv)?;
                        Ok(Message::ErrorXfer(text))
                    }
                    Msg::Info => {
                        let text = Self::decode_text(f.payload, iconv)?;
                        Ok(Message::Info(text))
                    }
                    Msg::Error => {
                        let text = Self::decode_text(f.payload, iconv)?;
                        Ok(Message::Error(text))
                    }
                    Msg::Warning => {
                        let text = Self::decode_text(f.payload, iconv)?;
                        Ok(Message::Warning(text))
                    }
                    Msg::ErrorSocket => {
                        let text = Self::decode_text(f.payload, iconv)?;
                        Ok(Message::ErrorSocket(text))
                    }
                    Msg::Log => {
                        let text = Self::decode_text(f.payload, iconv)?;
                        Ok(Message::Log(text))
                    }
                    Msg::Client => {
                        let text = Self::decode_text(f.payload, iconv)?;
                        Ok(Message::Client(text))
                    }
                    Msg::ErrorUtf8 => {
                        let text = Self::decode_text(f.payload, iconv)?;
                        Ok(Message::ErrorUtf8(text))
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
                    Msg::Redo => {
                        if f.payload.len() != 4 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid redo payload",
                            ));
                        }
                        let mut rdr = &f.payload[..];
                        let idx = rdr.read_u32::<BigEndian>()?;
                        Ok(Message::Redo(idx))
                    }
                    Msg::Stats => Ok(Message::Stats(f.payload)),
                    Msg::IoError => {
                        if f.payload.len() != 4 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid io_error payload",
                            ));
                        }
                        let mut rdr = &f.payload[..];
                        let val = rdr.read_u32::<BigEndian>()?;
                        Ok(Message::IoError(val))
                    }
                    Msg::IoTimeout => {
                        if f.payload.len() != 4 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid io_timeout payload",
                            ));
                        }
                        let mut rdr = &f.payload[..];
                        let val = rdr.read_u32::<BigEndian>()?;
                        Ok(Message::IoTimeout(val))
                    }
                    Msg::Noop => {
                        if !f.payload.is_empty() {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid noop payload",
                            ));
                        }
                        Ok(Message::Noop)
                    }
                    Msg::ErrorExit => {
                        if f.payload.len() != 1 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid error exit payload",
                            ));
                        }
                        Ok(Message::Exit(f.payload[0]))
                    }
                    Msg::Success => {
                        if f.payload.len() != 4 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid success payload",
                            ));
                        }
                        let mut rdr = &f.payload[..];
                        let idx = rdr.read_u32::<BigEndian>()?;
                        Ok(Message::Success(idx))
                    }
                    Msg::Deleted => {
                        if f.payload.len() != 4 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid deleted payload",
                            ));
                        }
                        let mut rdr = &f.payload[..];
                        let idx = rdr.read_u32::<BigEndian>()?;
                        Ok(Message::Deleted(idx))
                    }
                    Msg::NoSend => {
                        if f.payload.len() != 4 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid nosend payload",
                            ));
                        }
                        let mut rdr = &f.payload[..];
                        let idx = rdr.read_u32::<BigEndian>()?;
                        Ok(Message::NoSend(idx))
                    }
                    Msg::KeepAlive => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "unexpected keepalive message",
                    )),
                    other => Ok(Message::Other(other, f.payload)),
                }
            }
        }
    }

    pub fn from_file_list(
        entry: &FlistEntry,
        enc: &mut FlistEncoder,
        iconv: Option<&CharsetConv>,
    ) -> Self {
        let mut e = entry.clone();
        if let Some(cv) = iconv {
            e.path = cv.to_remote(&e.path).into_owned();
        }
        Message::FileListEntry(enc.encode_entry(&e))
    }

    pub fn to_file_list(
        &self,
        dec: &mut FlistDecoder,
        iconv: Option<&CharsetConv>,
    ) -> io::Result<FlistEntry> {
        match self {
            Message::FileListEntry(bytes) => {
                let mut e = dec
                    .decode_entry(bytes)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                if let Some(cv) = iconv {
                    e.path = cv.to_local(&e.path).into_owned();
                }
                Ok(e)
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not a file list entry",
            )),
        }
    }

    pub fn error_text(&self) -> Option<&str> {
        match self {
            Message::ErrorXfer(t)
            | Message::Error(t)
            | Message::ErrorSocket(t)
            | Message::ErrorUtf8(t) => Some(t),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_negotiation() {
        let latest = SUPPORTED_PROTOCOLS[0];
        for &peer in SUPPORTED_PROTOCOLS {
            assert_eq!(negotiate_version(latest, peer), Ok(peer));
            assert_eq!(negotiate_version(peer, latest), Ok(peer));
        }
        assert!(negotiate_version(latest, MIN_VERSION - 1).is_err());
    }

    #[test]
    fn frame_roundtrip() {
        let msg = Message::Data(b"hello".to_vec());
        let frame = msg.to_frame(7, None);
        let mut buf = Vec::new();
        frame.encode(&mut buf).unwrap();
        let decoded = Frame::decode(&buf[..]).unwrap();
        assert_eq!(decoded, frame);
        let msg2 = Message::from_frame(decoded, None).unwrap();
        assert_eq!(msg2, msg);

        let msg4 = Message::Data(b"1234".to_vec());
        let frame4 = msg4.to_frame(3, None);
        let mut buf4 = Vec::new();
        frame4.encode(&mut buf4).unwrap();
        let decoded4 = Frame::decode(&buf4[..]).unwrap();
        assert_eq!(decoded4, frame4);
        let msg4_round = Message::from_frame(decoded4, None).unwrap();
        assert_eq!(msg4_round, msg4);

        let msgc = Message::Codecs(vec![0, 1]);
        let framec = msgc.to_frame(1, None);
        let mut bufc = Vec::new();
        framec.encode(&mut bufc).unwrap();
        let decodedc = Frame::decode(&bufc[..]).unwrap();
        assert_eq!(decodedc, framec);
        let msgc_round = Message::from_frame(decodedc, None).unwrap();
        assert_eq!(msgc_round, msgc);
    }

    #[test]
    fn keepalive_frame() {
        let msg = Message::KeepAlive;
        let frame = msg.into_frame(0, None);
        assert_eq!(frame.header.tag, Tag::KeepAlive);
        assert_eq!(frame.header.msg, Msg::KeepAlive);
        let mut buf = Vec::new();
        frame.encode(&mut buf).unwrap();
        let decoded = Frame::decode(&buf[..]).unwrap();
        let msg2 = Message::from_frame(decoded, None).unwrap();
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
        assert!(Message::from_frame(frame, None).is_err());
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
        assert!(Message::from_frame(frame, None).is_err());
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
