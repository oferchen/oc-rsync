// crates/transport/src/tcp.rs
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;

use ipnet::IpNet;
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
        let addrs = addrs
            .into_iter()
            .filter(|a| match family {
                Some(AddressFamily::V4) => a.is_ipv4(),
                Some(AddressFamily::V6) => a.is_ipv6(),
                None => true,
            })
            .collect::<Vec<_>>();

        if addrs.is_empty() {
            return Err(io::Error::other("invalid address"));
        }

        let mut last_err = None;
        for addr in addrs {
            let stream_res = if let Some(dur) = connect_timeout {
                TcpStream::connect_timeout(&addr, dur)
            } else {
                TcpStream::connect(addr)
            };

            match stream_res {
                Ok(stream) => return Ok(Self { stream }),
                Err(e) => last_err = Some(e),
            }
        }

        Err(last_err.unwrap_or_else(|| io::Error::other("invalid address")))
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
            tracing::warn!(%addr, "rejected connection");
            let _ = stream.shutdown(std::net::Shutdown::Both);
            std::thread::sleep(Duration::from_millis(1));
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
        for opt in opts.iter() {
            match opt {
                SockOpt::KeepAlive(v) => sock.set_keepalive(*v)?,
                SockOpt::SendBuf(size) => sock.set_send_buffer_size(*size)?,
                SockOpt::RecvBuf(size) => sock.set_recv_buffer_size(*size)?,
                SockOpt::IpTtl(v) => {
                    #[cfg(any(
                        target_os = "android",
                        target_os = "fuchsia",
                        target_os = "illumos",
                        target_os = "linux",
                        target_os = "macos",
                        target_os = "ios",
                        target_os = "freebsd",
                        target_os = "netbsd",
                        target_os = "dragonfly",
                        target_os = "openbsd",
                        target_os = "windows",
                    ))]
                    {
                        sock.set_ttl(*v)?;
                    }
                    #[cfg(not(any(
                        target_os = "android",
                        target_os = "fuchsia",
                        target_os = "illumos",
                        target_os = "linux",
                        target_os = "macos",
                        target_os = "ios",
                        target_os = "freebsd",
                        target_os = "netbsd",
                        target_os = "dragonfly",
                        target_os = "openbsd",
                        target_os = "windows",
                    )))]
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            "IP_TTL not supported on this platform",
                        ));
                    }
                }
                SockOpt::IpTos(v) => {
                    #[cfg(not(any(
                        target_os = "fuchsia",
                        target_os = "redox",
                        target_os = "solaris",
                        target_os = "illumos",
                        target_os = "haiku",
                    )))]
                    {
                        sock.set_tos(*v)?;
                    }
                    #[cfg(any(
                        target_os = "fuchsia",
                        target_os = "redox",
                        target_os = "solaris",
                        target_os = "illumos",
                        target_os = "haiku",
                    ))]
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            "IP_TOS not supported on this platform",
                        ));
                    }
                }
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
                        target_os = "linux",
                    )))]
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            "SO_BINDTODEVICE is only supported on Linux-like systems",
                        ));
                    }
                }
                SockOpt::IpHopLimit(v) => {
                    #[cfg(any(
                        target_os = "android",
                        target_os = "fuchsia",
                        target_os = "illumos",
                        target_os = "linux",
                        target_os = "macos",
                        target_os = "ios",
                        target_os = "freebsd",
                        target_os = "netbsd",
                        target_os = "dragonfly",
                        target_os = "openbsd",
                        target_os = "windows",
                    ))]
                    {
                        sock.set_unicast_hops_v6(*v)?;
                    }
                    #[cfg(not(any(
                        target_os = "android",
                        target_os = "fuchsia",
                        target_os = "illumos",
                        target_os = "linux",
                        target_os = "macos",
                        target_os = "ios",
                        target_os = "freebsd",
                        target_os = "netbsd",
                        target_os = "dragonfly",
                        target_os = "openbsd",
                        target_os = "windows",
                    )))]
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            "IPV6_UNICAST_HOPS not supported on this platform",
                        ));
                    }
                }

                SockOpt::Linger(dur) => sock.set_linger(*dur)?,
                SockOpt::Broadcast(v) => sock.set_broadcast(*v)?,
                SockOpt::RcvTimeout(d) => sock.set_read_timeout(Some(*d))?,
                SockOpt::SndTimeout(d) => sock.set_write_timeout(Some(*d))?,
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
    if let Ok(net) = pat.parse::<IpNet>() {
        return net.contains(ip);
    }
    if let Ok(addr) = pat.parse::<IpAddr>() {
        return &addr == ip;
    }
    (pat, 0)
        .to_socket_addrs()
        .is_ok_and(|mut addrs| addrs.any(|a| &a.ip() == ip))
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
