// crates/transport/tests/retry.rs
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use transport::connect_with_retry;
use transport::tcp::TcpTransport;

#[test]
fn connect_eventually_succeeds_after_retry() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();
    drop(listener);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(25));
        let listener = TcpListener::bind(addr).expect("bind");
        let _ = listener.accept().unwrap();
    });

    TcpTransport::connect_with_retry(
        &addr.ip().to_string(),
        addr.port(),
        None,
        None,
        5,
        Duration::from_millis(10),
    )
    .expect("connect");
}

#[test]
fn connect_fails_after_retries() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let start = Instant::now();
    let res = TcpTransport::connect_with_retry(
        &addr.ip().to_string(),
        addr.port(),
        None,
        None,
        2,
        Duration::from_millis(10),
    );
    assert!(res.is_err());
    assert!(start.elapsed() >= Duration::from_millis(30));
}

#[test]
fn connect_waits_with_backoff_before_success() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();
    drop(listener);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(60));
        let listener = TcpListener::bind(addr).expect("bind");
        let _ = listener.accept().unwrap();
    });

    let start = Instant::now();
    connect_with_retry(
        &addr.ip().to_string(),
        addr.port(),
        None,
        None,
        5,
        Duration::from_millis(10),
    )
    .expect("connect");
    assert!(start.elapsed() >= Duration::from_millis(70));
}
