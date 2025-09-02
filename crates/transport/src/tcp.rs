// crates/transport/src/tcp.rs
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;

use socket2::SockRef;

use crate::{AddressFamily, DaemonTransport, SockOpt, Transport};

pub struct TcpTransport {
    stream: TcpStream,
}

impl TcpTransport {
    pub fn connect(
        host: &str,
        port: u16,
        connect_timeout: Option<Duration>,
        family: Option<AddressFamily>,
    ) -> io::Result<Self> {
        let addrs: Vec<SocketAddr> = (host, port).to_socket_addrs()?.collect();
        let addr = match family {
            Some(AddressFamily::V4) => addrs.iter().find(|a| a.is_ipv4()).copied(),
            Some(AddressFamily::V6) => addrs.iter().find(|a| a.is_ipv6()).copied(),
            None => addrs.into_iter().next(),
        }
        .ok_or_else(|| io::Error::other("invalid address"))?;

        let stream = if let Some(dur) = connect_timeout {
            TcpStream::connect_timeout(&addr, dur)?
        } else {
            TcpStream::connect(addr)?
        };

        Ok(Self { stream })
    }

    pub fn listen(
        addr: Option<IpAddr>,
        port: u16,
        family: Option<AddressFamily>,
    ) -> io::Result<(TcpListener, u16)> {
        let addr = match (addr, family) {
            (Some(ip), _) => ip,
            (None, Some(AddressFamily::V4)) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            (None, Some(AddressFamily::V6)) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
            (None, None) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        };
        let listener = TcpListener::bind((addr, port))?;
        let port = listener.local_addr()?.port();
        Ok((listener, port))
    }

    pub fn accept(
        listener: &TcpListener,
        hosts_allow: &[String],
        hosts_deny: &[String],
    ) -> io::Result<(TcpStream, SocketAddr)> {
        loop {
            let (stream, addr) = listener.accept()?;
            if host_allowed(&addr.ip(), hosts_allow, hosts_deny) {
                return Ok((stream, addr));
            }
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub fn authenticate(&mut self, token: Option<&str>, no_motd: bool) -> io::Result<()> {
        if no_motd {
            self.stream.write_all(&[0])?;
        }
        if let Some(tok) = token {
            self.stream.write_all(tok.as_bytes())?;
        }
        self.stream.write_all(b"\n")
    }

    pub fn apply_sockopts(&self, opts: &[SockOpt]) -> io::Result<()> {
        if opts.is_empty() {
            return Ok(());
        }
        let sock = SockRef::from(&self.stream);
        for opt in opts {
            match opt {
                SockOpt::KeepAlive(v) => sock.set_keepalive(*v)?,
                SockOpt::SendBuf(size) => sock.set_send_buffer_size(*size)?,
                SockOpt::RecvBuf(size) => sock.set_recv_buffer_size(*size)?,
                SockOpt::IpTtl(v) => sock.set_ttl(*v)?,
                SockOpt::IpTos(v) => sock.set_tos(*v)?,
                SockOpt::TcpNoDelay(v) => sock.set_nodelay(*v)?,
                SockOpt::ReuseAddr(v) => sock.set_reuse_address(*v)?,
                SockOpt::BindToDevice(iface) => {
                    #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
                    {
                        sock.bind_device(Some(iface.as_bytes()))?;
                    }
                    #[cfg(not(any(
                        target_os = "android",
                        target_os = "fuchsia",
                        target_os = "linux"
                    )))]
                    {
                        let _ = iface;
                    }
                }
                SockOpt::IpHopLimit(v) => sock.set_unicast_hops_v6(*v)?,
            }
        }
        Ok(())
    }

    pub fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.stream.set_read_timeout(dur)
    }

    pub fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.stream.set_write_timeout(dur)
    }
}

fn host_matches(ip: &IpAddr, pat: &str) -> bool {
    if pat == "*" {
        return true;
    }
    pat.parse::<IpAddr>().is_ok_and(|p| &p == ip)
}

fn host_allowed(ip: &IpAddr, allow: &[String], deny: &[String]) -> bool {
    if !allow.is_empty() && !allow.iter().any(|p| host_matches(ip, p)) {
        return false;
    }
    if deny.iter().any(|p| host_matches(ip, p)) {
        return false;
    }
    true
}

impl Transport for TcpTransport {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.stream.write_all(data).map_err(|e| {
            if e.kind() == io::ErrorKind::WouldBlock {
                io::Error::new(io::ErrorKind::TimedOut, "operation timed out")
            } else {
                e
            }
        })
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf).map_err(|e| {
            if e.kind() == io::ErrorKind::WouldBlock {
                io::Error::new(io::ErrorKind::TimedOut, "operation timed out")
            } else {
                e
            }
        })
    }

    fn set_read_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
        self.stream.set_read_timeout(dur)
    }

    fn set_write_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
        self.stream.set_write_timeout(dur)
    }
}

impl DaemonTransport for TcpTransport {}
