// crates/protocol/tests/protocol.rs
use protocol::{negotiate_version, Frame, Message, Msg, Tag, V31, V32};

#[test]
fn frame_roundtrip() {
    let msg = Message::Data(b"hello".to_vec());
    let frame = msg.to_frame(5);
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
}

#[test]
fn keepalive_roundtrip() {
    let msg = Message::KeepAlive;
    let frame = msg.to_frame(0);
    assert_eq!(frame.header.tag, Tag::KeepAlive);
    assert_eq!(frame.header.msg, Msg::KeepAlive);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let decoded = Frame::decode(&buf[..]).unwrap();
    assert_eq!(decoded, frame);
    let msg2 = Message::from_frame(decoded).unwrap();
    assert_eq!(msg2, Message::KeepAlive);
}

#[test]
fn version_negotiation() {
    assert_eq!(negotiate_version(V32, V32), Ok(V32));
    assert_eq!(negotiate_version(V32, V31), Ok(V31));
    assert_eq!(negotiate_version(V31, V32), Ok(V31));
    assert_eq!(negotiate_version(V31, V31), Ok(V31));
    assert!(negotiate_version(V32, 30).is_err());
}

#[test]
fn captured_frames_roundtrip() {
    const FILE_LIST: [u8; 16] = [
        0, 0, 0, 4, 0, 0, 0, 8, b'f', b'i', b'l', b'e', b'.', b't', b'x', b't',
    ];
    let frame = Frame::decode(&FILE_LIST[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::FileListEntry);
    let msg = Message::from_frame(frame.clone()).unwrap();
    assert_eq!(msg, Message::FileListEntry(b"file.txt".to_vec()));
    let mut buf = Vec::new();
    Message::FileListEntry(b"file.txt".to_vec())
        .into_frame(0)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, FILE_LIST);

    const ATTRS: [u8; 16] = [
        0, 0, 0, 5, 0, 0, 0, 8, b'm', b'o', b'd', b'e', b'=', b'7', b'5', b'5',
    ];
    let frame = Frame::decode(&ATTRS[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Attributes);
    let msg = Message::from_frame(frame.clone()).unwrap();
    assert_eq!(msg, Message::Attributes(b"mode=755".to_vec()));
    let mut buf = Vec::new();
    Message::Attributes(b"mode=755".to_vec())
        .into_frame(0)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, ATTRS);

    const ERR: [u8; 12] = [0, 0, 0, 6, 0, 0, 0, 4, b'o', b'o', b'p', b's'];
    let frame = Frame::decode(&ERR[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Error);
    let msg = Message::from_frame(frame.clone()).unwrap();
    assert_eq!(msg, Message::Error("oops".to_string()));
    let mut buf = Vec::new();
    Message::Error("oops".to_string())
        .into_frame(0)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, ERR);

    const PROG: [u8; 16] = [0, 0, 0, 7, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0x30, 0x39];
    let frame = Frame::decode(&PROG[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Progress);
    let msg = Message::from_frame(frame.clone()).unwrap();
    assert_eq!(msg, Message::Progress(0x3039));
    let mut buf = Vec::new();
    Message::Progress(0x3039)
        .into_frame(0)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, PROG);
}
