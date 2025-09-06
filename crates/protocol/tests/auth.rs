// crates/protocol/tests/auth.rs
use checksums::{StrongHash, strong_digest};
use compress::{available_codecs, encode_codecs};
use protocol::{Message, SUPPORTED_CAPS, SUPPORTED_PROTOCOLS, Server};
use std::io::Cursor;
use std::time::Duration;

const CHALLENGE: [u8; 16] = *b"0123456789abcdef";

#[test]
fn server_accepts_valid_challenge() {
    let local = available_codecs();
    let payload = encode_codecs(&local);
    let frame = Message::Codecs(payload.clone()).to_frame(0, None);
    let mut frame_buf = Vec::new();
    frame.encode(&mut frame_buf).unwrap();
    let latest = SUPPORTED_PROTOCOLS[0];
    let token = "secret";
    let mut data = CHALLENGE.to_vec();
    data.extend_from_slice(token.as_bytes());
    let resp = strong_digest(&data, StrongHash::Md5, 0);
    let mut input = Cursor::new({
        let mut v = vec![0, 0];
        v.extend_from_slice(&resp);
        v.extend_from_slice(&latest.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&frame_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let (caps, peer_codecs) = srv
        .handshake(latest, SUPPORTED_CAPS, &local, Some(token))
        .unwrap();
    assert_eq!(caps, SUPPORTED_CAPS);
    assert_eq!(peer_codecs, local);
    let expected = {
        let mut v = CHALLENGE.to_vec();
        v.extend_from_slice(&latest.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        let mut out_frame = Vec::new();
        frame.encode(&mut out_frame).unwrap();
        v.extend_from_slice(&out_frame);
        v
    };
    assert_eq!(output, expected);
}

#[test]
fn server_rejects_invalid_challenge() {
    let local = available_codecs();
    let payload = encode_codecs(&local);
    let frame = Message::Codecs(payload.clone()).to_frame(0, None);
    let mut frame_buf = Vec::new();
    frame.encode(&mut frame_buf).unwrap();
    let latest = SUPPORTED_PROTOCOLS[0];
    let token = "secret";
    let mut input = Cursor::new({
        let mut v = vec![0, 0];
        v.extend_from_slice(&[0u8; 16]);
        v.extend_from_slice(&latest.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&frame_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let err = srv
        .handshake(latest, SUPPORTED_CAPS, &local, Some(token))
        .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    assert_eq!(output, CHALLENGE.to_vec());
}

#[test]
fn server_rejects_mismatched_token_constant_time() {
    let local = available_codecs();
    let payload = encode_codecs(&local);
    let frame = Message::Codecs(payload.clone()).to_frame(0, None);
    let mut frame_buf = Vec::new();
    frame.encode(&mut frame_buf).unwrap();
    let latest = SUPPORTED_PROTOCOLS[0];
    let server_token = "secret";
    let wrong_token = "not_secret";
    let mut data = CHALLENGE.to_vec();
    data.extend_from_slice(wrong_token.as_bytes());
    let resp = strong_digest(&data, StrongHash::Md5, 0);
    let mut input = Cursor::new({
        let mut v = vec![0, 0];
        v.extend_from_slice(&resp);
        v.extend_from_slice(&latest.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&frame_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let err = srv
        .handshake(latest, SUPPORTED_CAPS, &local, Some(server_token))
        .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    assert_eq!(output, CHALLENGE.to_vec());
}
