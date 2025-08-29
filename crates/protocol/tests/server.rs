use compress::{available_codecs, encode_codecs};
use protocol::{Server, LATEST_VERSION};
use std::io::Cursor;

#[test]
fn server_negotiates_version() {
    let payload = encode_codecs(available_codecs());
    let mut input = Cursor::new({
        let mut v = LATEST_VERSION.to_be_bytes().to_vec();
        v.push(payload.len() as u8);
        v.extend_from_slice(&payload);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output);
    let peer_codecs = srv.handshake().unwrap();
    assert_eq!(srv.version, LATEST_VERSION);
    assert_eq!(peer_codecs, available_codecs());
    let expected = {
        let mut v = LATEST_VERSION.to_be_bytes().to_vec();
        v.push(payload.len() as u8);
        v.extend_from_slice(&payload);
        v
    };
    assert_eq!(output, expected);
}
