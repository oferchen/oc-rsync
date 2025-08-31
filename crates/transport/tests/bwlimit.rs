// crates/transport/tests/bwlimit.rs
use std::io;
use std::time::{Duration, Instant};

use transport::{LocalPipeTransport, RateLimitedTransport, Transport};

#[test]
fn sustained_transfer_is_limited() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let mut t = RateLimitedTransport::new(inner, 1024);
    let data = vec![0u8; 2048];
    let start = Instant::now();
    t.send(&data).unwrap();
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(1900));
}

#[test]
fn short_burst_is_initially_allowed() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let mut t = RateLimitedTransport::new(inner, 1024);

    let small = vec![0u8; 50];
    let start = Instant::now();
    t.send(&small).unwrap();
    assert!(start.elapsed() < Duration::from_millis(100));

    let large = vec![0u8; 974];
    let start2 = Instant::now();
    t.send(&large).unwrap();
    let elapsed = start2.elapsed();
    assert!(elapsed >= Duration::from_millis(900));
}
