// crates/transport/src/rate.rs
use std::io;
use std::time::{Duration, Instant};

use crate::Transport;

pub struct RateLimitedTransport<T> {
    inner: T,
    bwlimit: u64,
    debt: i64,
    prior: Option<Instant>,
}

impl<T> RateLimitedTransport<T> {
    pub fn new(inner: T, bwlimit: u64) -> Self {
        Self {
            inner,
            bwlimit,
            debt: 0,
            prior: None,
        }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Transport> Transport for RateLimitedTransport<T> {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        const ONE_SEC_MICROS: i64 = 1_000_000;
        const MIN_SLEEP: i64 = ONE_SEC_MICROS / 10; // 100ms

        self.inner.send(data)?;

        self.debt += data.len() as i64;
        let now = Instant::now();
        if let Some(prior) = self.prior {
            let elapsed_us = now.duration_since(prior).as_micros() as i64;
            let allowance = elapsed_us * self.bwlimit as i64 / ONE_SEC_MICROS;
            self.debt -= allowance;
            if self.debt < 0 {
                self.debt = 0;
            }
        }

        let sleep_us = self.debt * ONE_SEC_MICROS / self.bwlimit as i64;
        if sleep_us >= MIN_SLEEP {
            std::thread::sleep(Duration::from_micros(sleep_us as u64));
            let after = Instant::now();
            let slept_us = after.duration_since(now).as_micros() as i64;
            let leftover = sleep_us - slept_us;
            self.debt = leftover * self.bwlimit as i64 / ONE_SEC_MICROS;
            self.prior = Some(after);
        } else {
            self.prior = Some(now);
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
