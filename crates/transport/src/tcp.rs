use std::io::{self, Read, Write};
use std::net::TcpStream;

use crate::{DaemonTransport, Transport};

/// Transport over a TCP stream to an rsync daemon.
pub struct TcpTransport {
    stream: TcpStream,
}

impl TcpTransport {
    /// Connect to the given address and return a TCP transport.
    pub fn connect(addr: &str) -> io::Result<Self> {
        Ok(Self {
            stream: TcpStream::connect(addr)?,
        })
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
