// tests/timeout.rs
use std::io;
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use protocol::Demux;
use transport::{TcpTransport, Transport};

#[test]
fn tcp_read_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (_sock, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_secs(5));
    });
    let mut t = TcpTransport::connect(&addr.ip().to_string(), addr.port(), None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let mut buf = [0u8; 1];
    let err = t.receive(&mut buf).err().expect("error");
    assert!(err.kind() == io::ErrorKind::WouldBlock || err.kind() == io::ErrorKind::TimedOut);
}

#[test]
fn demux_channel_timeout() {
    let mut demux = Demux::new(Duration::from_millis(100));
    demux.register_channel(0);
    thread::sleep(Duration::from_millis(200));
    let err = demux.poll().unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
}
