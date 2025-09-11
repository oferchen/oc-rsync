// crates/protocol/src/types/codes.rs
use std::fmt;
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Tag {
    Message = 0,
    KeepAlive = 1,
    Data = 2,
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
            2 => Ok(Tag::Data),
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
