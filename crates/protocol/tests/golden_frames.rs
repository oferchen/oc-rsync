// crates/protocol/tests/golden_frames.rs
use protocol::{Frame, Message, Msg, Tag};

#[test]
fn decode_version_golden() {
    const VERSION: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 31];
    let frame = Frame::decode(&VERSION[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Version);
    let msg = Message::from_frame(frame).unwrap();
    assert_eq!(msg, Message::Version(31));
}

#[test]
fn decode_keepalive_golden() {
    const KEEPALIVE: [u8; 8] = [0, 0, 1, 3, 0, 0, 0, 0];
    let frame = Frame::decode(&KEEPALIVE[..]).unwrap();
    assert_eq!(frame.header.tag, Tag::KeepAlive);
    let msg = Message::from_frame(frame).unwrap();
    assert_eq!(msg, Message::KeepAlive);
}

#[test]
fn decode_progress_golden() {
    const PROG: [u8; 16] = [0, 0, 0, 7, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0x30, 0x39];
    let frame = Frame::decode(&PROG[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Progress);
    let msg = Message::from_frame(frame).unwrap();
    assert_eq!(msg, Message::Progress(0x3039));
}

#[test]
fn decode_error_golden() {
    const ERR: [u8; 12] = [0, 0, 0, 6, 0, 0, 0, 4, b'o', b'o', b'p', b's'];
    let frame = Frame::decode(&ERR[..]).unwrap();
    assert_eq!(frame.header.msg, Msg::Error);
    let msg = Message::from_frame(frame).unwrap();
    assert_eq!(msg, Message::Error("oops".to_string()));
}
