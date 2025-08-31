// crates/transport/src/lib.rs
use std::io::{self, Read, Write};

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

#[derive(Debug, Clone, Copy)]
pub enum SockOpt {
    KeepAlive(bool),
    SendBuf(usize),
    RecvBuf(usize),
}

pub fn parse_sockopts(opts: &[String]) -> Result<Vec<SockOpt>, String> {
    opts.iter().map(|s| parse_sockopt(s)).collect()
}

fn parse_sockopt(s: &str) -> Result<SockOpt, String> {
    let (name, value) = match s.split_once('=') {
        Some((n, v)) => (n.trim(), Some(v.trim())),
        None => (s.trim(), None),
    };
    match name {
        "SO_KEEPALIVE" => {
            let enabled = value.map(|v| v != "0").unwrap_or(true);
            Ok(SockOpt::KeepAlive(enabled))
        }
        "SO_SNDBUF" => {
            let v = value.ok_or_else(|| "SO_SNDBUF requires a value".to_string())?;
            let size = v
                .parse::<usize>()
                .map_err(|_| "invalid SO_SNDBUF value".to_string())?;
            Ok(SockOpt::SendBuf(size))
        }
        "SO_RCVBUF" => {
            let v = value.ok_or_else(|| "SO_RCVBUF requires a value".to_string())?;
            let size = v
                .parse::<usize>()
                .map_err(|_| "invalid SO_RCVBUF value".to_string())?;
            Ok(SockOpt::RecvBuf(size))
        }
        _ => Err(format!("unknown socket option: {name}")),
    }
}
