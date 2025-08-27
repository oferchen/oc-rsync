use std::io::Cursor;

use transport::{LocalPipeTransport, Transport};

#[test]
fn send_writes_to_writer() {
    let reader = Cursor::new(Vec::new());
    let writer = Cursor::new(Vec::new());
    let mut transport = LocalPipeTransport::new(reader, writer);

    transport.send(b"hello").expect("send should succeed");

    let (_, writer) = transport.into_inner();
    assert_eq!(writer.into_inner(), b"hello");
}

#[test]
fn receive_reads_from_reader() {
    let reader = Cursor::new(b"world".to_vec());
    let writer = Cursor::new(Vec::new());
    let mut transport = LocalPipeTransport::new(reader, writer);

    let mut buf = [0u8; 5];
    let n = transport.receive(&mut buf).expect("receive should succeed");

    assert_eq!(n, 5);
    assert_eq!(&buf, b"world");
}
