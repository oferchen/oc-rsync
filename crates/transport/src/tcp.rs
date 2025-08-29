use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::{DaemonTransport, Transport};

/// Transport over a TCP stream to an rsync daemon.
pub struct TcpTransport {
    stream: TcpStream,
}

impl TcpTransport {
    /// Connect to the given address and return a TCP transport.
    pub fn connect(addr: &str, timeout: Option<Duration>) -> io::Result<Self> {
        let stream = if let Some(dur) = timeout {
            let addr = addr
                .to_socket_addrs()?
                .next()
                .ok_or_else(|| io::Error::other("invalid address"))?;
            TcpStream::connect_timeout(&addr, dur)?
        } else {
            TcpStream::connect(addr)?
        };
        Ok(Self { stream })
    }

    /// Create a transport from an existing `TcpStream`.
    pub fn from_stream(stream: TcpStream) -> Self {
        Self { stream }
    }

    /// Send an authentication token terminated by a newline. If `token` is
    /// `None` an empty line is sent which typically causes the daemon to
    /// reject the connection when authentication is required.
    pub fn authenticate(&mut self, token: Option<&str>) -> io::Result<()> {
        if let Some(tok) = token {
            self.stream.write_all(tok.as_bytes())?;
        }
        self.stream.write_all(b"\n")
    }

    /// Configure a read timeout on the underlying `TcpStream`.
    pub fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.stream.set_read_timeout(dur)
    }
}

impl Transport for TcpTransport {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.stream.write_all(data)
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}

impl DaemonTransport for TcpTransport {}
