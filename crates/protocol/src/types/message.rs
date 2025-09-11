// crates/protocol/src/types/message.rs
use byteorder::{BigEndian, ReadBytesExt};
use filelist::{Decoder as FlistDecoder, Encoder as FlistEncoder, Entry as FlistEntry};
use std::io;

use crate::frames::{Frame, FrameHeader};

use super::{CharsetConv, Msg, Tag};

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
                    tag: Tag::Data,
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
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::ErrorExit,
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
            Tag::Data => {
                if f.header.msg != Msg::Data {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid data message",
                    ));
                }
                Ok(Message::Data(f.payload))
            }
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
            Ok(cv.decode_remote(&payload).to_string())
        } else {
            String::from_utf8(payload).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
    }
}
