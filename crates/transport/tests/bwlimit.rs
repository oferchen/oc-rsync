// crates/transport/tests/bwlimit.rs
use std::io;
use std::time::{Duration, Instant};

use transport::{rate_limited, LocalPipeTransport, Transport};

#[test]
fn sustained_transfer_is_limited() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let mut t = rate_limited(inner, 1024);
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
    let mut t = rate_limited(inner, 1024);

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

#[test]
fn idle_time_refills_bucket() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let mut t = rate_limited(inner, 1024);

    let block = vec![0u8; 1024];
    t.send(&block).unwrap();
    std::thread::sleep(Duration::from_millis(1100));

    let start = Instant::now();
    t.send(&block).unwrap();

    assert!(start.elapsed() < Duration::from_millis(150));
}

#[test]
fn partial_refill_shortens_sleep() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let mut t = rate_limited(inner, 1024);

    let block = vec![0u8; 1024];
    t.send(&block).unwrap();

    std::thread::sleep(Duration::from_millis(500));

    let start = Instant::now();
    t.send(&block).unwrap();
    let elapsed = start.elapsed();

    assert!(elapsed >= Duration::from_millis(400));
    assert!(elapsed < Duration::from_millis(800));
}
