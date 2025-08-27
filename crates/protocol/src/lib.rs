use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::convert::TryFrom;
use std::fmt;
use std::io::{self, Read, Write};

/// Latest protocol version supported by this implementation.
pub const LATEST_VERSION: u32 = 31;
/// Oldest protocol version we support.
pub const MIN_VERSION: u32 = 29;

/// Error returned when there is no version overlap during negotiation.
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

/// Negotiate protocol version with peer.
///
/// Returns the agreed version or [`VersionError`] if there is no overlap.
pub fn negotiate_version(peer: u32) -> Result<u32, VersionError> {
    if peer >= LATEST_VERSION {
        Ok(LATEST_VERSION)
    } else if peer >= MIN_VERSION {
        Ok(peer)
    } else {
        Err(VersionError(peer))
    }
}

/// Tags used to multiplex streams.
///
/// `Tag` differentiates between control frames such as keepalive messages and
/// regular protocol messages.  Individual message variants are further
/// identified by [`Msg`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Tag {
    Message = 0,
    KeepAlive = 1,
}

/// Error returned when attempting to convert from an unknown tag value.
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

/// Identifier for the type of [`Message`] contained in a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Msg {
    Version = 0,
    Data = 1,
    Done = 2,
}

impl From<u8> for Msg {
    fn from(v: u8) -> Self {
        match v {
            0 => Msg::Version,
            1 => Msg::Data,
            2 => Msg::Done,
            _ => Msg::Data,
        }
    }
}

/// Header for a [`Frame`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    pub channel: u16,
    pub tag: Tag,
    /// Identifier for the contained [`Message`]. This allows payloads of any
    /// length (including 4 bytes) without ambiguity between [`Message::Version`]
    /// and [`Message::Data`].
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
        let msg = Msg::from(r.read_u8()?);
        let len = r.read_u32::<BigEndian>()?;
        Ok(FrameHeader {
            channel,
            tag,
            msg,
            len,
        })
    }
}

/// A frame of data sent over the wire.
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

/// High level messages encoded inside frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Version(u32),
    Data(Vec<u8>),
    Done,
    KeepAlive,
}

impl Message {
    pub fn to_frame(&self, channel: u16) -> Frame {
        match self {
            Message::Version(v) => {
                let mut payload = Vec::new();
                payload.write_u32::<BigEndian>(*v).unwrap();
                let header = FrameHeader {
                    channel,
                    tag: Tag::Message,
                    msg: Msg::Version,
                    len: payload.len() as u32,
                };
                Frame { header, payload }
            }
            Message::Data(data) => {
                let payload = data.clone();
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
                    msg: Msg::Data, // unused
                    len: 0,
                };
                Frame { header, payload }
            }
        }
    }

    pub fn from_frame(f: Frame) -> io::Result<Self> {
        match f.header.tag {
            Tag::KeepAlive => Ok(Message::KeepAlive),
            Tag::Message => match f.header.msg {
                Msg::Version => {
                    let mut rdr = &f.payload[..];
                    let v = rdr.read_u32::<BigEndian>()?;
                    Ok(Message::Version(v))
                }
                Msg::Data => Ok(Message::Data(f.payload)),
                Msg::Done => Ok(Message::Done),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_negotiation() {
        assert_eq!(negotiate_version(40), Ok(31));
        assert_eq!(negotiate_version(31), Ok(31));
        assert_eq!(negotiate_version(30), Ok(30));
        assert!(negotiate_version(28).is_err());
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

        // A 4-byte payload should not be interpreted as a version message
        let msg4 = Message::Data(b"1234".to_vec());
        let frame4 = msg4.to_frame(3);
        let mut buf4 = Vec::new();
        frame4.encode(&mut buf4).unwrap();
        let decoded4 = Frame::decode(&buf4[..]).unwrap();
        assert_eq!(decoded4, frame4);
        let msg4_round = Message::from_frame(decoded4).unwrap();
        assert_eq!(msg4_round, msg4);
    }

    #[test]
    fn keepalive_frame() {
        let msg = Message::KeepAlive;
        let frame = msg.to_frame(0);
        let mut buf = Vec::new();
        frame.encode(&mut buf).unwrap();
        let decoded = Frame::decode(&buf[..]).unwrap();
        let msg2 = Message::from_frame(decoded).unwrap();
        assert_eq!(msg2, Message::KeepAlive);
    }

    #[test]
    fn unknown_tag_errors() {
        // channel:0, tag:99 (invalid), len:0
        let buf = [0u8, 0, 99, 0, 0, 0, 0];
        assert!(Frame::decode(&buf[..]).is_err());
    }
}
