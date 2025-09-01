// crates/transport/tests/bwlimit.rs
use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::time::{Duration, Instant};

use transport::{rate_limited, LocalPipeTransport, RateLimitedTransport, Transport};

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

#[test]
fn burst_and_sleep_cross_check() {
    use std::io::Write;

    let reader = io::empty();
    let counts = Rc::new(RefCell::new(Vec::new()));

    struct Recorder {
        inner: Vec<u8>,
        counts: Rc<RefCell<Vec<usize>>>,
    }

    impl Write for Recorder {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.counts.borrow_mut().push(buf.len());
            self.inner.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let writer = Recorder {
        inner: Vec::new(),
        counts: counts.clone(),
    };
    let inner = LocalPipeTransport::new(reader, writer);
    let sleeps = Rc::new(RefCell::new(Vec::new()));
    let sleep_rec = sleeps.clone();
    let sleeper = move |d: Duration| {
        sleep_rec.borrow_mut().push(d);
    };
    let mut t = RateLimitedTransport::with_sleeper(inner, 4, Box::new(sleeper));

    let data = vec![0u8; 1536];
    t.send(&data).unwrap();

    assert_eq!(&*counts.borrow(), &[512, 512, 512]);
    let expected = [
        Duration::from_secs(128),
        Duration::from_secs(256),
        Duration::from_secs(384),
    ];
    assert_eq!(&*sleeps.borrow(), &expected);
}
