// crates/transport/src/lib.rs
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

mod rate;
pub mod ssh;
pub mod tcp;

pub use rate::RateLimitedTransport;
pub use ssh::SshStdioTransport;
pub use tcp::TcpTransport;

/// Wrap a [`Transport`] with a bandwidth limiter using rsync's
/// token-bucket algorithm.
pub fn rate_limited<T: Transport>(inner: T, bwlimit: u64) -> RateLimitedTransport<T> {
    RateLimitedTransport::new(inner, bwlimit)
}

#[derive(Clone, Copy, Debug)]
pub enum AddressFamily {
    V4,
    V6,
}

pub trait Transport {
    fn send(&mut self, data: &[u8]) -> io::Result<()>;

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize>;

    fn set_read_timeout(&mut self, _dur: Option<Duration>) -> io::Result<()> {
        Ok(())
    }

    fn set_write_timeout(&mut self, _dur: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
}

pub struct LocalPipeTransport<R, W> {
    reader: R,
    writer: W,
}

impl<R, W> LocalPipeTransport<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }

    pub fn into_inner(self) -> (R, W) {
        (self.reader, self.writer)
    }

    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }
}

impl<R: Read, W: Write> Transport for LocalPipeTransport<R, W> {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

/// Wraps a [`Transport`] and enforces a maximum idle duration for the
/// connection. Any call to [`Transport::send`] or [`Transport::receive`]
/// that occurs after the timeout has elapsed will return
/// [`io::ErrorKind::TimedOut`].
pub struct TimeoutTransport<T> {
    inner: T,
    timeout: Duration,
    last: Instant,
}

impl<T> TimeoutTransport<T> {
    /// Create a new [`TimeoutTransport`] that fails if no I/O occurs within
    /// `timeout`.
    pub fn new(inner: T, timeout: Duration) -> Self {
        Self {
            inner,
            timeout,
            last: Instant::now(),
        }
    }

    /// Consume the wrapper and return the inner transport.
    pub fn into_inner(self) -> T {
        self.inner
    }

    fn check_timeout(&self) -> io::Result<()> {
        if self.last.elapsed() > self.timeout {
            Err(io::Error::new(io::ErrorKind::TimedOut, "connection timed out"))
        } else {
            Ok(())
        }
    }
}

impl<T: Transport> Transport for TimeoutTransport<T> {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.check_timeout()?;
        self.inner.send(data)?;
        self.last = Instant::now();
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.check_timeout()?;
        let n = self.inner.receive(buf)?;
        if n > 0 {
            self.last = Instant::now();
        }
        Ok(n)
    }
}

pub trait SshTransport: Transport {}

pub trait DaemonTransport: Transport {}
