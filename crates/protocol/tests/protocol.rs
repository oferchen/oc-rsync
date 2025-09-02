// crates/protocol/tests/protocol.rs
use encoding_rs::Encoding;
use filelist::{Decoder as FDecoder, Encoder as FEncoder, Entry as FEntry};
use protocol::{
    negotiate_version, CharsetConv, Frame, Message, Msg, Tag, MIN_VERSION, SUPPORTED_PROTOCOLS,
};

#[test]
fn frame_roundtrip() {
    let msg = Message::Data(b"hello".to_vec());
    let frame = msg.to_frame(5, None);
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
}

#[test]
fn keepalive_roundtrip() {
    let msg = Message::KeepAlive;
    let frame = msg.to_frame(0, None);
    assert_eq!(frame.header.tag, Tag::KeepAlive);
    assert_eq!(frame.header.msg, Msg::KeepAlive);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let decoded = Frame::decode(&buf[..]).unwrap();
    assert_eq!(decoded, frame);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, Message::KeepAlive);
}

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
fn captured_frames_roundtrip() {
    let entry = FEntry {
        path: b"file.txt".to_vec(),
        uid: 0,
        gid: 0,
        group: None,
    };
    let mut fenc = FEncoder::new();
    let payload = fenc.encode_entry(&entry);
    let mut expected = Vec::new();
    Frame {
        header: protocol::FrameHeader {
            channel: 0,
            tag: Tag::Message,
            msg: Msg::FileListEntry,
            len: payload.len() as u32,
        },
        payload: payload.clone(),
    }
    .encode(&mut expected)
    .unwrap();
    let frame = Frame::decode(&expected[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::FileListEntry);
    let msg = Message::from_frame(frame.clone(), None).unwrap();
    assert_eq!(msg, Message::FileListEntry(payload.clone()));
    let mut buf = Vec::new();
    Message::FileListEntry(payload.clone())
        .into_frame(0, None)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, expected);
    let mut fdec = FDecoder::new();
    assert_eq!(msg.to_file_list(&mut fdec, None).unwrap(), entry);

    const ATTRS: [u8; 16] = [
        0, 0, 0, 244, 0, 0, 0, 8, b'm', b'o', b'd', b'e', b'=', b'7', b'5', b'5',
    ];
    let frame = Frame::decode(&ATTRS[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Attributes);
    let msg = Message::from_frame(frame.clone(), None).unwrap();
    assert_eq!(msg, Message::Attributes(b"mode=755".to_vec()));
    let mut buf = Vec::new();
    Message::Attributes(b"mode=755".to_vec())
        .into_frame(0, None)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, ATTRS);

    const ERR: [u8; 12] = [0, 0, 0, 3, 0, 0, 0, 4, b'o', b'o', b'p', b's'];
    let frame = Frame::decode(&ERR[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Error);
    let msg = Message::from_frame(frame.clone(), None).unwrap();
    assert_eq!(msg, Message::Error("oops".to_string()));
    let mut buf = Vec::new();
    Message::Error("oops".to_string())
        .into_frame(0, None)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, ERR);

    const PROG: [u8; 16] = [0, 0, 0, 245, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0x30, 0x39];
    let frame = Frame::decode(&PROG[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Progress);
    let msg = Message::from_frame(frame.clone(), None).unwrap();
    assert_eq!(msg, Message::Progress(0x3039));
    let mut buf = Vec::new();
    Message::Progress(0x3039)
        .into_frame(0, None)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, PROG);
}

#[test]
fn extra_messages_roundtrip() {
    let msg = Message::Redo(0x01020304);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let decoded = Frame::decode(&buf[..]).unwrap();
    assert_eq!(decoded.header.msg, Msg::Redo);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::Stats(vec![1, 2, 3]);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let decoded = Frame::decode(&buf[..]).unwrap();
    assert_eq!(decoded.header.msg, Msg::Stats);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::Success(7);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let decoded = Frame::decode(&buf[..]).unwrap();
    assert_eq!(decoded.header.msg, Msg::Success);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::Deleted(9);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let decoded = Frame::decode(&buf[..]).unwrap();
    assert_eq!(decoded.header.msg, Msg::Deleted);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::NoSend(11);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let decoded = Frame::decode(&buf[..]).unwrap();
    assert_eq!(decoded.header.msg, Msg::NoSend);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::ErrorExit(2);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let decoded = Frame::decode(&buf[..]).unwrap();
    assert_eq!(decoded.header.msg, Msg::ErrorExit);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);
}

#[test]
fn error_message_iconv_roundtrip() {
    let cv = CharsetConv::new(Encoding::for_label(b"latin1").unwrap());
    let msg = Message::Error("Grüße".into());
    let frame = msg.to_frame(0, Some(&cv));
    let decoded = Message::from_frame(frame, Some(&cv)).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn filelist_iconv_roundtrip() {
    let cv = CharsetConv::new(Encoding::for_label(b"latin1").unwrap());
    let entry = FEntry {
        path: "Grüße".as_bytes().to_vec(),
        uid: 0,
        gid: 0,
        group: None,
    };
    let mut enc = FEncoder::new();
    let msg = Message::from_file_list(&entry, &mut enc, Some(&cv));
    let mut dec = FDecoder::new();
    let round = msg.to_file_list(&mut dec, Some(&cv)).unwrap();
    assert_eq!(round, entry);
}
