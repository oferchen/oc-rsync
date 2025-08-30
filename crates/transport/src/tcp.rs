// crates/transport/src/tcp.rs
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::{AddressFamily, DaemonTransport, Transport};

pub struct TcpTransport {
    stream: TcpStream,
}

impl TcpTransport {
    pub fn connect(
        host: &str,
        port: u16,
        timeout: Option<Duration>,
        family: Option<AddressFamily>,
    ) -> io::Result<Self> {
        let addrs: Vec<SocketAddr> = (host, port).to_socket_addrs()?.collect();
        let addr = match family {
            Some(AddressFamily::V4) => addrs.iter().find(|a| a.is_ipv4()).copied(),
            Some(AddressFamily::V6) => addrs.iter().find(|a| a.is_ipv6()).copied(),
            None => addrs.into_iter().next(),
        }
        .ok_or_else(|| io::Error::other("invalid address"))?;
        let stream = if let Some(dur) = timeout {
            TcpStream::connect_timeout(&addr, dur)?
        } else {
            TcpStream::connect(addr)?
        };
        Ok(Self { stream })
    }

    pub fn listen(addr: Option<IpAddr>, port: u16) -> io::Result<(TcpListener, u16)> {
        let addr = addr.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        let listener = TcpListener::bind((addr, port))?;
        let port = listener.local_addr()?.port();
        Ok((listener, port))
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub fn authenticate(&mut self, token: Option<&str>) -> io::Result<()> {
        if let Some(tok) = token {
            self.stream.write_all(tok.as_bytes())?;
        }
        self.stream.write_all(b"\n")
    }

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
