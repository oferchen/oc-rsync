use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use transport::{tcp::TcpTransport, Transport};

#[test]
fn send_receive_over_tcp() {
    // Start a simple echo server.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).unwrap();
        stream.write_all(&buf).unwrap();
    });

    let mut transport = TcpTransport::connect(&addr.to_string()).expect("connect");
    transport.send(b"ping").expect("send");
    let mut buf = [0u8; 4];
    let n = transport.receive(&mut buf).expect("receive");
    assert_eq!(n, 4);
    assert_eq!(&buf, b"ping");
}
