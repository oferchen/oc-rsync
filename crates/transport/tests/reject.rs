// crates/transport/tests/reject.rs
use std::net::{Ipv4Addr, Ipv6Addr, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use transport::{tcp::TcpTransport, AddressFamily};

#[test]
fn rejects_sleep_prevents_spin() {
    // Bind an IPv6 listener so we can connect via IPv4 and IPv6.
    let (listener, port) = TcpTransport::listen(None, 0, Some(AddressFamily::V6)).expect("listen");
    let accept_listener = listener.try_clone().expect("clone");

    let handle = thread::spawn(move || {
        TcpTransport::accept(&accept_listener, &[], &["127.0.0.1".to_string()]).expect("accept");
    });

    // Ensure the accept thread is ready.
    thread::sleep(Duration::from_millis(10));

    let start = Instant::now();
    for _ in 0..5 {
        let _ = TcpStream::connect((Ipv4Addr::LOCALHOST, port));
    }
    // Final allowed connection via IPv6 to exit the accept loop.
    let _ = TcpStream::connect((Ipv6Addr::LOCALHOST, port));
    handle.join().unwrap();

    assert!(start.elapsed() >= Duration::from_millis(5));
}
