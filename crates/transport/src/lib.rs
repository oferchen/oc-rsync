// crates/transport/src/lib.rs
use std::io::{self, Read, Write};
use std::time::Duration;

mod rate;
pub mod ssh;
pub mod tcp;

pub use rate::RateLimitedTransport;
pub use ssh::SshStdioTransport;
pub use tcp::TcpTransport;

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

pub trait SshTransport: Transport {}

pub trait DaemonTransport: Transport {}
