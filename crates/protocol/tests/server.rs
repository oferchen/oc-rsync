use compress::{available_codecs, encode_codecs};
use protocol::{Server, CAP_CODECS, LATEST_VERSION};
use std::io::Cursor;

#[test]
fn server_negotiates_version() {
    let payload = encode_codecs(available_codecs());
    let codecs_frame = protocol::Message::Codecs(payload.clone()).to_frame(0);
    let mut codecs_buf = Vec::new();
    codecs_frame.encode(&mut codecs_buf).unwrap();
    let mut input = Cursor::new({
        let mut v = LATEST_VERSION.to_be_bytes().to_vec();
        v.extend_from_slice(&CAP_CODECS.to_be_bytes());
        v.extend_from_slice(&codecs_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output);
    let peer_codecs = srv.handshake().unwrap();
    assert_eq!(srv.version, LATEST_VERSION);
    assert_eq!(peer_codecs, available_codecs());
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
