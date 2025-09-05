// tests/transport.rs

use std::io::Write;
use std::net::{TcpListener, ToSocketAddrs};
use std::thread;
use std::time::Duration;

use transport::{AddressFamily, TcpTransport, Transport};

#[test]
fn tcp_prefers_ipv4() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.write_all(b"4").unwrap();
    });
    let mut t =
        TcpTransport::connect("localhost", port, None, None, Some(AddressFamily::V4)).unwrap();
    let mut buf = [0u8; 1];
    t.receive(&mut buf).unwrap();
    assert_eq!(&buf, b"4");
    assert!(TcpTransport::connect("localhost", port, None, None, Some(AddressFamily::V6)).is_err());
}

#[test]
fn tcp_prefers_ipv6() {
    let listener = match TcpListener::bind("[::1]:0") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("IPv6 not available: {e}");
            return;
        }
    };
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.write_all(b"6").unwrap();
    });
    let mut t =
        TcpTransport::connect("localhost", port, None, None, Some(AddressFamily::V6)).unwrap();
    let mut buf = [0u8; 1];
    t.receive(&mut buf).unwrap();
    assert_eq!(&buf, b"6");
    assert!(TcpTransport::connect("localhost", port, None, None, Some(AddressFamily::V4)).is_err());
}

#[test]
fn tcp_retries_addresses() {
    if ("localhost", 0)
        .to_socket_addrs()
        .map(|iter| iter.count())
        .unwrap_or(0)
        < 2
    {
        eprintln!("localhost does not resolve to multiple addresses; skipping");
        return;
    }

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.write_all(b"r").unwrap();
    });

    let mut t =
        TcpTransport::connect("localhost", port, Some(Duration::from_secs(1)), None, None).unwrap();
    let mut buf = [0u8; 1];
    t.receive(&mut buf).unwrap();
    assert_eq!(&buf, b"r");
}
