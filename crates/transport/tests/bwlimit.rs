// crates/transport/tests/bwlimit.rs
use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::time::Duration;

use transport::{LocalPipeTransport, RateLimitedTransport};

#[test]
fn write_below_min_sleep_is_unthrottled() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let sleeps = Rc::new(RefCell::new(Vec::new()));
    let rec = sleeps.clone();
    let sleeper = move |d: Duration| {
        rec.borrow_mut().push(d);
    };
    let mut t = RateLimitedTransport::with_sleeper(inner, 4 * 1024, Box::new(sleeper));
    let data = vec![0u8; 256];
    t.send(&data).unwrap();
    assert!(sleeps.borrow().is_empty());
}

#[test]
fn sustained_transfer_is_limited() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let sleeps = Rc::new(RefCell::new(Vec::new()));
    let rec = sleeps.clone();
    let sleeper = move |d: Duration| {
        rec.borrow_mut().push(d);
    };
    let mut t = RateLimitedTransport::with_sleeper(inner, 4 * 1024, Box::new(sleeper));
    let data = vec![0u8; 8 * 1024];
    t.send(&data).unwrap();
    let sleeps = sleeps.borrow();
    assert_eq!(sleeps.len(), 1);
    assert!(sleeps[0] >= Duration::from_secs(2));
}

#[test]
fn idle_time_refills_debt() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let sleeps = Rc::new(RefCell::new(Vec::new()));
    let rec = sleeps.clone();
    let sleeper = move |d: Duration| {
        rec.borrow_mut().push(d);
    };
    let mut t = RateLimitedTransport::with_sleeper(inner, 4 * 1024, Box::new(sleeper));
    let block = vec![0u8; 8 * 1024];
    t.send(&block).unwrap();
    std::thread::sleep(Duration::from_secs(3));
    let small = vec![0u8; 256];
    t.send(&small).unwrap();
    let sleeps = sleeps.borrow();
    assert_eq!(sleeps.len(), 1);
    assert!(sleeps[0] >= Duration::from_secs(2));
}

#[test]
fn partial_refill_shortens_sleep() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let sleeps = Rc::new(RefCell::new(Vec::new()));
    let rec = sleeps.clone();
    let sleeper = move |d: Duration| {
        rec.borrow_mut().push(d);
    };
    let mut t = RateLimitedTransport::with_sleeper(inner, 4 * 1024, Box::new(sleeper));
    let first = vec![0u8; 8 * 1024];
    t.send(&first).unwrap();
    std::thread::sleep(Duration::from_secs(1));
    let second = vec![0u8; 1024];
    t.send(&second).unwrap();
    let sleeps = sleeps.borrow();
    assert_eq!(sleeps.len(), 2);
    assert!(sleeps[0] >= Duration::from_secs(2));
    assert!(sleeps[1] >= Duration::from_millis(1200));
    assert!(sleeps[1] < Duration::from_millis(1300));
}

fn upstream_simulation(bwlimit: u64, writes: &[usize]) -> Vec<Duration> {
    const ONE_SEC: u64 = 1_000_000;
    const MIN_SLEEP: u64 = ONE_SEC / 10;
    let mut total = 0u64;
    let mut sleeps = Vec::new();
    for &w in writes {
        total += w as u64;
        let sleep_us = total * ONE_SEC / bwlimit;
        if sleep_us >= MIN_SLEEP {
            sleeps.push(Duration::from_micros(sleep_us));
            total = 0;
        }
    }
    sleeps
}

#[test]
fn parity_with_upstream_reference() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let sleeps = Rc::new(RefCell::new(Vec::new()));
    let rec = sleeps.clone();
    let sleeper = move |d: Duration| {
        rec.borrow_mut().push(d);
        std::thread::sleep(d);
    };
    let bw = 4 * 1024;
    let mut t = RateLimitedTransport::with_sleeper(inner, bw, Box::new(sleeper));
    let block = vec![0u8; 512];
    for _ in 0..3 {
        t.send(&block).unwrap();
    }
    let expected = upstream_simulation(bw, &[512, 512, 512]);
    assert_eq!(&*sleeps.borrow(), &expected);
}
