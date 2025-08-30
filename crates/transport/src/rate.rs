// crates/transport/src/rate.rs
use std::io;
use std::time::{Duration, Instant};

use crate::Transport;

pub struct RateLimitedTransport<T> {
    inner: T,
    bwlimit: u64,
    start: Instant,
    sent: u64,
}

impl<T> RateLimitedTransport<T> {
    pub fn new(inner: T, bwlimit: u64) -> Self {
        Self {
            inner,
            bwlimit,
            start: Instant::now(),
            sent: 0,
        }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Transport> Transport for RateLimitedTransport<T> {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.inner.send(data)?;
        self.sent += data.len() as u64;
        let elapsed = self.start.elapsed();
        let expected = Duration::from_secs_f64(self.sent as f64 / self.bwlimit as f64);
        if expected > elapsed {
            std::thread::sleep(expected - elapsed);
        }
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.receive(buf)
    }
}
