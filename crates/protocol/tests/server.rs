// crates/protocol/tests/server.rs
use compress::{available_codecs, encode_codecs};
use protocol::{Server, CAP_CODECS, LATEST_VERSION};
use std::io::Cursor;
use std::time::Duration;

#[test]
fn server_negotiates_version() {
    let local = available_codecs(false);
    let payload = encode_codecs(&local);
    let codecs_frame = protocol::Message::Codecs(payload.clone()).to_frame(0);
    let mut codecs_buf = Vec::new();
    codecs_frame.encode(&mut codecs_buf).unwrap();
    let mut input = Cursor::new({
        let mut v = vec![0];
        v.extend_from_slice(&LATEST_VERSION.to_be_bytes());
        v.extend_from_slice(&CAP_CODECS.to_be_bytes());
        v.extend_from_slice(&codecs_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let peer_codecs = srv.handshake(&local).unwrap();
    assert_eq!(srv.version, LATEST_VERSION);
    assert_eq!(peer_codecs, local);
    let expected = {
        let mut v = LATEST_VERSION.to_be_bytes().to_vec();
        v.extend_from_slice(&CAP_CODECS.to_be_bytes());
        let mut out_frame = Vec::new();
        codecs_frame.encode(&mut out_frame).unwrap();
        v.extend_from_slice(&out_frame);
        v
    };
    assert_eq!(output, expected);
}
