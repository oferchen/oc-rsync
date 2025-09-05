// crates/transport/src/tcp.rs
use std::io::{self, Read, Write};
use std::net::{
    IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs,
};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::time::Duration;

use nix::poll::{poll, PollFd, PollFlags, PollTimeout};

use ipnet::IpNet;
use socket2::SockRef;

use crate::{AddressFamily, DaemonTransport, SockOpt, Transport};

pub struct TcpTransport {
    stream: TcpStream,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
    blocking_io: bool,
}

impl TcpTransport {
    pub fn connect(
        host: &str,
        port: u16,
        connect_timeout: Option<Duration>,
        timeout: Option<Duration>,
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
                Ok(stream) => {
                    if let Some(dur) = timeout {
                        let _ = stream.set_read_timeout(Some(dur));
                        let _ = stream.set_write_timeout(Some(dur));
                    }
                    let _ = stream.set_nonblocking(true);
                    return Ok(Self {
                        stream,
                        read_timeout: timeout,
                        write_timeout: timeout,
                        blocking_io: false,
                    });
                }
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
        let _ = stream.set_nonblocking(true);
        Self {
            stream,
            read_timeout: None,
            write_timeout: None,
            blocking_io: false,
        }
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
                        let _ = iface;
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
        if !self.blocking_io {
            let fd = self.stream.as_raw_fd();
            wait_fd(fd, PollFlags::POLLOUT, self.write_timeout)?;
        }
        self.stream.write_all(data).map_err(|e| {
            if e.kind() == io::ErrorKind::WouldBlock {
                io::Error::new(io::ErrorKind::TimedOut, "operation timed out")
            } else {
                e
            }
        })
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.blocking_io {
            let fd = self.stream.as_raw_fd();
            wait_fd(fd, PollFlags::POLLIN, self.read_timeout)?;
        }
        self.stream.read(buf).map_err(|e| {
            if e.kind() == io::ErrorKind::WouldBlock {
                io::Error::new(io::ErrorKind::TimedOut, "operation timed out")
            } else {
                e
            }
        })
    }

    fn set_read_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
        self.read_timeout = dur;
        if self.blocking_io {
            self.stream.set_read_timeout(dur)?;
        } else {
            self.stream.set_read_timeout(None)?;
        }
        Ok(())
    }

    fn set_write_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
        self.write_timeout = dur;
        if self.blocking_io {
            self.stream.set_write_timeout(dur)?;
        } else {
            self.stream.set_write_timeout(None)?;
        }
        Ok(())
    }

    fn close(&mut self) -> io::Result<()> {
        self.stream.shutdown(Shutdown::Both)
    }
}

impl DaemonTransport for TcpTransport {}

impl TcpTransport {
    pub fn set_blocking_io(&mut self, blocking: bool) -> io::Result<()> {
        self.blocking_io = blocking;
        self.stream.set_nonblocking(!blocking)?;
        if blocking {
            self.stream.set_read_timeout(self.read_timeout)?;
            self.stream.set_write_timeout(self.write_timeout)?;
        } else {
            self.stream.set_read_timeout(None)?;
            self.stream.set_write_timeout(None)?;
        }
        Ok(())
    }

    pub fn into_inner(self) -> TcpStream {
        self.stream
    }
}

#[doc = "Borrow a file descriptor, ensuring it is non-negative.\n\n\
# Safety\n\
The caller must guarantee that `fd` refers to a valid open file descriptor for the\
duration of the returned `BorrowedFd`."]
fn borrow_fd(fd: RawFd) -> io::Result<BorrowedFd<'static>> {
    if fd < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "file descriptor must be non-negative",
        ));
    }
    Ok(unsafe { BorrowedFd::borrow_raw(fd) })
}

fn wait_fd(fd: RawFd, flags: PollFlags, timeout: Option<Duration>) -> io::Result<()> {
    let timeout = match timeout {
        Some(dur) => {
            PollTimeout::try_from(dur).map_err(|_| io::Error::other("timeout overflow"))?
        }
        None => PollTimeout::NONE,
    };
    let mut fds = [PollFd::new(borrow_fd(fd)?, flags)];
    let res = poll(&mut fds, timeout).map_err(io::Error::from)?;
    if res == 0 {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "operation timed out",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_fd_invalid_fd() {
        let err = wait_fd(-1, PollFlags::POLLIN, None).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
