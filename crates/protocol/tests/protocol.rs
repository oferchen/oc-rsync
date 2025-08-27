use protocol::{negotiate_version, Frame, Message, Msg, Tag};

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

    // A 4-byte payload should not be misinterpreted as a version message
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
    assert_eq!(negotiate_version(40), Ok(31));
    assert_eq!(negotiate_version(31), Ok(31));
    assert_eq!(negotiate_version(30), Ok(30));
    assert!(negotiate_version(28).is_err());
}
