
pub mod protocol {
    // Placeholder for the protocol crate.
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

/// Latest protocol version supported by this implementation.
pub const LATEST_VERSION: u32 = 31;

/// Negotiate protocol version with peer. Returns agreed version or `None`
/// if there is no overlap.
pub fn negotiate_version(peer: u32) -> Option<u32> {
    if peer >= LATEST_VERSION {
        Some(LATEST_VERSION)
    } else if peer >= 29 { // minimum we support
        Some(peer)
    } else {
        None
    }
}

/// Tags used to multiplex streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Tag {
    Message = 0,
    KeepAlive = 1,
}

impl From<u8> for Tag {
    fn from(v: u8) -> Self {
        match v {
            0 => Tag::Message,
            1 => Tag::KeepAlive,
            _ => Tag::Message,
        }
    }
}

/// A frame of data sent over the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub channel: u16,
    pub tag: Tag,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn encode<W: Write>(&self, mut w: W) -> io::Result<()> {
        w.write_u16::<BigEndian>(self.channel)?;
        w.write_u8(self.tag as u8)?;
        w.write_u32::<BigEndian>(self.payload.len() as u32)?;
        w.write_all(&self.payload)
    }

    pub fn decode<R: Read>(mut r: R) -> io::Result<Self> {
        let channel = r.read_u16::<BigEndian>()?;
        let tag = Tag::from(r.read_u8()?);
        let len = r.read_u32::<BigEndian>()? as usize;
        let mut payload = vec![0; len];
        r.read_exact(&mut payload)?;
        Ok(Frame { channel, tag, payload })
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
                Frame { channel, tag: Tag::Message, payload }
            }
            Message::Data(data) => Frame { channel, tag: Tag::Message, payload: data.clone() },
            Message::Done => Frame { channel, tag: Tag::Message, payload: vec![] },
            Message::KeepAlive => Frame { channel, tag: Tag::KeepAlive, payload: vec![] },
        }
    }

    pub fn from_frame(f: Frame) -> io::Result<Self> {
        match f.tag {
            Tag::KeepAlive => Ok(Message::KeepAlive),
            Tag::Message => {
                if f.payload.is_empty() {
                    Ok(Message::Done)
                } else if f.payload.len() == 4 {
                    let mut rdr = &f.payload[..];
                    let v = rdr.read_u32::<BigEndian>()?;
                    Ok(Message::Version(v))
                } else {
                    Ok(Message::Data(f.payload))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_negotiation() {
        assert_eq!(negotiate_version(40), Some(31));
        assert_eq!(negotiate_version(31), Some(31));
        assert_eq!(negotiate_version(30), Some(30));
        assert_eq!(negotiate_version(28), None);
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
}
