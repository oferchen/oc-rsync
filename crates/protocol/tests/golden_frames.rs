// crates/protocol/tests/golden_frames.rs
use protocol::{Frame, Message, Msg, Tag};

#[test]
fn decode_version_golden() {
    const VERSION: [u8; 12] = [0, 0, 0, 240, 0, 0, 0, 4, 0, 0, 0, 32];
    let frame = Frame::decode(&VERSION[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Version);
    let msg = Message::from_frame(frame, None).unwrap();
    assert_eq!(msg, Message::Version(32));
}

#[test]
fn decode_keepalive_golden() {
    const KEEPALIVE: [u8; 8] = [0, 0, 1, 242, 0, 0, 0, 0];
    let frame = Frame::decode(&KEEPALIVE[..]).unwrap();
    assert_eq!(frame.header.tag, Tag::KeepAlive);
    let msg = Message::from_frame(frame, None).unwrap();
    assert_eq!(msg, Message::KeepAlive);
}

#[test]
fn decode_progress_golden() {
    const PROG: [u8; 16] = [0, 0, 0, 245, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0x30, 0x39];
    let frame = Frame::decode(&PROG[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Progress);
    let msg = Message::from_frame(frame, None).unwrap();
    assert_eq!(msg, Message::Progress(0x3039));
}

#[test]
fn decode_xattrs_golden() {
    const XATTRS: [u8; 14] = [
        0, 0, 0, 0xF7, 0, 0, 0, 6, b'u', b's', b'e', b'r', b'=', b'1',
    ];
    let frame = Frame::decode(&XATTRS[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Xattrs);
    let msg = Message::from_frame(frame, None).unwrap();
    assert_eq!(msg, Message::Xattrs(b"user=1".to_vec()));
}

#[test]
fn decode_error_golden() {
    const ERR: [u8; 12] = [0, 0, 0, 3, 0, 0, 0, 4, b'o', b'o', b'p', b's'];
    let frame = Frame::decode(&ERR[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Error);
    let msg = Message::from_frame(frame, None).unwrap();
    assert_eq!(msg, Message::Error("oops".to_string()));
}
