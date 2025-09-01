// crates/transport/tests/bwlimit.rs
use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::time::Duration;

use transport::{LocalPipeTransport, RateLimitedTransport};

#[test]
fn short_transfer_below_burst_is_unthrottled() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let sleeps = Rc::new(RefCell::new(Vec::new()));
    let rec = sleeps.clone();
    let sleeper = move |d: Duration| {
        rec.borrow_mut().push(d);
    };
    let mut t = RateLimitedTransport::with_sleeper(inner, 1024, Box::new(sleeper));
    let data = vec![0u8; 1024];
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
    let mut t = RateLimitedTransport::with_sleeper(inner, 1024, Box::new(sleeper));
    let burst = 1024 * 128;
    let data = vec![0u8; burst as usize + 2048];
    t.send(&data).unwrap();
    assert_eq!(sleeps.borrow().len(), 1);
    assert!(sleeps.borrow()[0] >= Duration::from_millis(1900));
}

#[test]
fn idle_time_refills_bucket() {
    let reader = io::empty();
    let writer = Vec::new();
    let inner = LocalPipeTransport::new(reader, writer);
    let sleeps = Rc::new(RefCell::new(Vec::new()));
    let rec = sleeps.clone();
    let sleeper = move |d: Duration| {
        rec.borrow_mut().push(d);
    };
    let mut t = RateLimitedTransport::with_sleeper(inner, 1024, Box::new(sleeper));
    let burst = 1024 * 128;
    let block = vec![0u8; burst as usize];
    t.send(&block).unwrap();
    std::thread::sleep(Duration::from_millis(1100));
    let small = vec![0u8; 1024];
    t.send(&small).unwrap();
    assert!(sleeps.borrow().is_empty());
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
    let mut t = RateLimitedTransport::with_sleeper(inner, 1024, Box::new(sleeper));
    let burst = 1024 * 128;
    let block = vec![0u8; burst as usize];
    t.send(&block).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    let small = vec![0u8; 1024];
    t.send(&small).unwrap();
    let sleeps = sleeps.borrow();
    assert_eq!(sleeps.len(), 1);
    assert!(sleeps[0] >= Duration::from_millis(400));
    assert!(sleeps[0] < Duration::from_millis(800));
}

#[test]
fn matches_upstream_trace() {
    use std::io::Write;

    const UPSTREAM_WRITES: &[usize] = &[512, 512, 512];
    const UPSTREAM_SLEEPS: &[Duration] = &[Duration::from_secs(128), Duration::from_secs(128)];

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

    assert_eq!(&*counts.borrow(), UPSTREAM_WRITES);
    assert_eq!(&*sleeps.borrow(), UPSTREAM_SLEEPS);
}
