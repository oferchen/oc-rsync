// crates/transport/src/rate.rs
use std::io;
use std::time::{Duration, Instant};

use crate::Transport;

pub struct RateLimitedTransport<T> {
    inner: T,
    bwlimit: u64,
    backlog: u64,
    prior: Option<Instant>,
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
            backlog: 0,
            prior: None,
            burst,
            sleeper,
        }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    fn regulate(&mut self, sent: u64) {
        const ONE_SEC: u64 = 1_000_000;
        const MIN_SLEEP: u64 = ONE_SEC / 10;

        self.backlog += sent;
        let start = Instant::now();
        if let Some(prior) = self.prior {
            let elapsed_us = start.duration_since(prior).as_micros() as u64;
            let allowance = elapsed_us.saturating_mul(self.bwlimit) / ONE_SEC;
            self.backlog = self.backlog.saturating_sub(allowance);
        }

        let sleep_us = self.backlog.saturating_mul(ONE_SEC) / self.bwlimit;
        if sleep_us < MIN_SLEEP {
            self.prior = Some(start);
            return;
        }

        (self.sleeper)(Duration::from_micros(sleep_us));
        let after = Instant::now();
        let elapsed_us = after.duration_since(start).as_micros() as u64;
        let leftover = sleep_us.saturating_sub(elapsed_us);
        self.backlog = leftover.saturating_mul(self.bwlimit) / ONE_SEC;
        self.prior = Some(after);
    }
}

impl<T: Transport> Transport for RateLimitedTransport<T> {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        let mut offset = 0;
        while offset < data.len() {
            let end = std::cmp::min(offset + self.burst, data.len());
            self.inner.send(&data[offset..end])?;
            self.regulate((end - offset) as u64);
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
