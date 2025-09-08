// crates/transport/src/stdio.rs
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

use crate::Transport;

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

    fn close(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

pub struct TimeoutTransport<T> {
    inner: T,
    timeout: Option<Duration>,
    last: Instant,
}

impl<T: Transport> TimeoutTransport<T> {
    pub fn new(mut inner: T, timeout: Option<Duration>) -> io::Result<Self> {
        if let Some(dur) = timeout {
            inner.set_read_timeout(Some(dur))?;
            inner.set_write_timeout(Some(dur))?;
        }
        Ok(Self {
            inner,
            timeout,
            last: Instant::now(),
        })
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn refresh(&mut self) {
        self.last = Instant::now();
    }

    fn check_timeout(&self) -> io::Result<()> {
        if let Some(dur) = self.timeout
            && self.last.elapsed() >= dur
        {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "connection timed out",
            ));
        }
        Ok(())
    }
}

impl<T: Transport> Transport for TimeoutTransport<T> {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.check_timeout()?;
        self.inner.send(data)?;
        self.refresh();
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.check_timeout()?;
        let n = self.inner.receive(buf)?;
        if n > 0 {
            self.refresh();
        }
        Ok(n)
    }

    fn set_read_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
        self.inner.set_read_timeout(dur)?;
        self.timeout = dur;
        Ok(())
    }

    fn set_write_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
        self.inner.set_write_timeout(dur)
    }

    fn update_timeout(&mut self) {
        self.refresh();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor};

    #[test]
    fn timeout_errors_at_exact_duration() {
        let reader = Cursor::new(Vec::new());
        let writer = Cursor::new(Vec::new());
        let dur = Duration::from_millis(100);
        let mut t =
            TimeoutTransport::new(LocalPipeTransport::new(reader, writer), Some(dur)).unwrap();

        t.last = Instant::now() - dur + Duration::from_millis(1);
        t.send(b"ok").unwrap();

        t.last = Instant::now() - dur;
        let err = t.send(b"fail").expect_err("timeout error");
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
    }
}
