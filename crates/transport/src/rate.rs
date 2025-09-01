// crates/transport/src/rate.rs
use std::io;
use std::time::{Duration, Instant};

use crate::Transport;

pub struct RateLimitedTransport<T> {
    inner: T,
    bwlimit: u64,
    tokens: i64,
    last: Instant,
    burst: i64,
    sleeper: Box<dyn Fn(Duration)>,
}

impl<T> RateLimitedTransport<T> {
    pub fn new(inner: T, bwlimit: u64) -> Self {
        Self::with_sleeper(inner, bwlimit, Box::new(std::thread::sleep))
    }

    #[doc(hidden)]
    pub fn with_sleeper(inner: T, bwlimit: u64, sleeper: Box<dyn Fn(Duration)>) -> Self {
        let burst = std::cmp::max(bwlimit * 128, 512) as i64;
        Self {
            inner,
            bwlimit,
            tokens: burst,
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
    fn replenish(&mut self) {
        const ONE_SEC: i64 = 1_000_000;
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_micros() as i64;
        let added = elapsed * self.bwlimit as i64 / ONE_SEC;
        self.tokens = (self.tokens + added).min(self.burst);
        self.last = now;
    }

    fn regulate(&mut self, need: usize) -> io::Result<()> {
        const ONE_SEC: i64 = 1_000_000;
        const MIN_SLEEP: i64 = ONE_SEC / 10;

        let mut need = need as i64;
        loop {
            self.replenish();
            if self.tokens >= need {
                self.tokens -= need;
                return Ok(());
            }
            let deficit = need - self.tokens;
            let sleep_us = deficit * ONE_SEC / self.bwlimit as i64;
            if sleep_us < MIN_SLEEP {
                self.tokens -= need;
                return Ok(());
            }
            let start = Instant::now();
            (self.sleeper)(Duration::from_micros(sleep_us as u64));
            self.inner.update_timeout();
            self.last = start;
            self.tokens -= need;
            need = 0;
        }
    }
}

impl<T: Transport> Transport for RateLimitedTransport<T> {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        let mut offset = 0;
        while offset < data.len() {
            let end = std::cmp::min(offset + self.burst as usize, data.len());
            let chunk = end - offset;
            self.regulate(chunk)?;
            self.inner.send(&data[offset..end])?;
            offset = end;
        }
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.receive(buf)
    }

    fn set_read_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
        self.inner.set_read_timeout(dur)
    }

    fn set_write_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
        self.inner.set_write_timeout(dur)
    }
}
