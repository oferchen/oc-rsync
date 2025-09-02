// crates/transport/src/rate.rs
use std::time::{Duration, Instant};

use crate::Transport;

pub struct RateLimitedTransport<T> {
    inner: T,
    bwlimit: u64,
    debt: i64,
    last: Instant,
    burst: usize,
    sleeper: Box<dyn Fn(Duration)>,
}

impl<T> RateLimitedTransport<T> {
    pub fn new(inner: T, bwlimit: u64) -> Self {
        Self::with_sleeper(inner, bwlimit, Box::new(std::thread::sleep))
    }

    #[doc(hidden)]
    pub fn with_sleeper(inner: T, bwlimit: u64, sleeper: Box<dyn Fn(Duration)>) -> Self {
        let burst = std::cmp::max(bwlimit * 128, 512) as usize;
        Self {
            inner,
            bwlimit,
            debt: 0,
            last: Instant::now(),
            burst,
            sleeper,
        }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Transport> RateLimitedTransport<T> {
    fn throttle(&mut self, wrote: usize) {
        const ONE_SEC: i64 = 1_000_000;
        const MIN_SLEEP: i64 = ONE_SEC / 10;

        self.debt += wrote as i64;
        let start = Instant::now();
        let elapsed = start.duration_since(self.last).as_micros() as i64;
        if elapsed > 0 {
            self.debt -= elapsed * self.bwlimit as i64 / ONE_SEC;
            if self.debt < 0 {
                self.debt = 0;
            }
        }

        let sleep_us = self.debt * ONE_SEC / self.bwlimit as i64;
        if sleep_us >= MIN_SLEEP {
            (self.sleeper)(Duration::from_micros(sleep_us as u64));
            self.inner.update_timeout();
            let actual = Instant::now().duration_since(start).as_micros() as i64;
            self.debt = (sleep_us - actual) * self.bwlimit as i64 / ONE_SEC;
            self.last = Instant::now();
        } else {
            self.last = start;
        }
    }
}

impl<T: Transport> Transport for RateLimitedTransport<T> {
    fn send(&mut self, data: &[u8]) -> std::io::Result<()> {
        let mut offset = 0;
        while offset < data.len() {
            let end = std::cmp::min(offset + self.burst, data.len());
            let chunk = end - offset;
            self.inner.send(&data[offset..end])?;
            self.throttle(chunk);
            offset = end;
        }
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.receive(buf)
    }

    fn set_read_timeout(&mut self, dur: Option<Duration>) -> std::io::Result<()> {
        self.inner.set_read_timeout(dur)
    }

    fn set_write_timeout(&mut self, dur: Option<Duration>) -> std::io::Result<()> {
        self.inner.set_write_timeout(dur)
    }
}
