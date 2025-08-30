// crates/transport/tests/bwlimit.rs
use std::io;
use std::time::{Duration, Instant};

use transport::{LocalPipeTransport, RateLimitedTransport, Transport};

#[test]
fn rate_limited_transport_caps_speed() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let mut t = RateLimitedTransport::new(inner, 1024);
    let data = vec![0u8; 1024];
    let start = Instant::now();
    t.send(&data).unwrap();
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(900));
}
