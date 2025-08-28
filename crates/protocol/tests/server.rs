use protocol::{Server, LATEST_VERSION};
use std::io::{Cursor, Read, Write};

#[test]
fn server_negotiates_version() {
    let mut input = Cursor::new(LATEST_VERSION.to_be_bytes().to_vec());
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output);
    let ver = srv.handshake().unwrap();
    assert_eq!(ver, LATEST_VERSION);
    assert_eq!(output, LATEST_VERSION.to_be_bytes());
}
