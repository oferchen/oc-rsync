use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::{AddressFamily, DaemonTransport, Transport};

/// Transport over a TCP stream to an rsync daemon.
pub struct TcpTransport {
    stream: TcpStream,
}

impl TcpTransport {
    /// Connect to the given host and port and return a TCP transport.
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

    /// Create a TCP listener bound to the given address and port.
    ///
    /// Returns the listener along with the actual port it was bound to.  When
    /// `port` is set to 0 the operating system will select a free ephemeral
    /// port which is reported in the returned tuple.  This mirrors the
    /// behaviour of `rsyncd.conf` where specifying `port 0` asks the daemon to
    /// bind to any available port.
    pub fn listen(
        addr: Option<IpAddr>,
        port: u16,
        family: Option<AddressFamily>,
    ) -> io::Result<(TcpListener, u16)> {
        let addr = match (addr, family) {
            (Some(a), _) => a,
            (None, Some(AddressFamily::V4)) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            (None, Some(AddressFamily::V6)) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
            (None, None) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        };
        let listener = TcpListener::bind((addr, port))?;
        let port = listener.local_addr()?.port();
        Ok((listener, port))
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
