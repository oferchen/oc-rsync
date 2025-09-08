use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

use crate::types::{Msg, Tag};

pub trait FrameCodec: Sized {
    fn encode<W: Write>(&self, w: &mut W) -> io::Result<()>;
    fn decode<R: Read>(r: &mut R) -> io::Result<Self>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    pub channel: u16,
    pub tag: Tag,
    pub msg: Msg,
    pub len: u32,
}

impl FrameHeader {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u16::<BigEndian>(self.channel)?;
        w.write_u8(self.tag as u8)?;
        w.write_u8(self.msg as u8)?;
        w.write_u32::<BigEndian>(self.len)?;
        Ok(())
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
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

impl FrameCodec for Frame {
    fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.header.encode(w)?;
        w.write_all(&self.payload)
    }

    fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        let header = FrameHeader::decode(r)?;
        let mut payload = vec![0; header.len as usize];
        r.read_exact(&mut payload)?;
        Ok(Frame { header, payload })
    }
}

impl Frame {
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        <Self as FrameCodec>::encode(self, w)
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
        <Self as FrameCodec>::decode(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Message;

    #[test]
    fn frame_roundtrip() {
        let msg = Message::Data(b"hello".to_vec());
        let frame = msg.to_frame(7, None);
        assert_eq!(frame.header.tag, Tag::Data);
        let mut buf = Vec::new();
        frame.encode(&mut buf).unwrap();
        let decoded = Frame::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded.header.tag, Tag::Data);
        assert_eq!(decoded, frame);
        let msg2 = Message::from_frame(decoded, None).unwrap();
        assert_eq!(msg2, msg);

        let msg4 = Message::Data(b"1234".to_vec());
        let frame4 = msg4.to_frame(3, None);
        assert_eq!(frame4.header.tag, Tag::Data);
        let mut buf4 = Vec::new();
        frame4.encode(&mut buf4).unwrap();
        let decoded4 = Frame::decode(&mut buf4.as_slice()).unwrap();
        assert_eq!(decoded4.header.tag, Tag::Data);
        assert_eq!(decoded4, frame4);
        let msg4_round = Message::from_frame(decoded4, None).unwrap();
        assert_eq!(msg4_round, msg4);

        let msgc = Message::Codecs(vec![0, 1]);
        let framec = msgc.to_frame(1, None);
        let mut bufc = Vec::new();
        framec.encode(&mut bufc).unwrap();
        let decodedc = Frame::decode(&mut bufc.as_slice()).unwrap();
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
        let decoded = Frame::decode(&mut buf.as_slice()).unwrap();
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
        assert!(Frame::decode(&mut &buf[..]).is_err());
    }

    #[test]
    fn unknown_msg_errors() {
        let buf = [0u8, 0, 0, 99, 0, 0, 0, 0];
        assert!(Frame::decode(&mut &buf[..]).is_err());
    }

    #[test]
    fn truncated_header_errors() {
        let buf = [0u8, 0, 0];
        assert!(Frame::decode(&mut &buf[..]).is_err());
    }

    #[test]
    fn truncated_payload_errors() {
        let header = FrameHeader {
            channel: 0,
            tag: Tag::Message,
            msg: Msg::Data,
            len: 5,
        };
        let mut buf = Vec::new();
        header.encode(&mut buf).unwrap();
        buf.extend_from_slice(&[1, 2]);
        assert!(Frame::decode(&mut buf.as_slice()).is_err());
    }
}
