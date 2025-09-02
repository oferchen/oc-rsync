// crates/transport/src/lib.rs
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

mod rate;
pub mod ssh;
pub mod tcp;

pub use rate::RateLimitedTransport;
pub use ssh::SshStdioTransport;
pub use tcp::TcpTransport;

pub fn rate_limited<T: Transport>(inner: T, bwlimit: u64) -> RateLimitedTransport<T> {
    RateLimitedTransport::new(inner, bwlimit)
}

pub fn pipe<S, D>(src: &mut S, dst: &mut D) -> io::Result<()>
where
    S: Transport,
    D: Transport,
{
    let mut buf = [0u8; 8192];
    loop {
        let n = src.receive(&mut buf)?;
        if n == 0 {
            break;
        }
        dst.send(&buf[..n])?;
    }
    Ok(())
}

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

    fn update_timeout(&mut self) {}
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
        if let Some(dur) = self.timeout {
            if self.last.elapsed() > dur {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "connection timed out",
                ));
            }
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

pub trait SshTransport: Transport {}

pub trait DaemonTransport: Transport {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SockOpt {
    KeepAlive(bool),
    SendBuf(usize),
    RecvBuf(usize),
    IpTtl(u32),
    IpTos(u32),
    TcpNoDelay(bool),
    ReuseAddr(bool),
    BindToDevice(String),
    IpHopLimit(u32),
}

pub fn parse_sockopts(opts: &[String]) -> Result<Vec<SockOpt>, String> {
    opts.iter().map(|s| parse_sockopt(s)).collect()
}

fn parse_sockopt(s: &str) -> Result<SockOpt, String> {
    if let Some((prefix, rest)) = s.split_once(':') {
        return parse_prefixed_sockopt(prefix, rest);
    }

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
        "TCP_NODELAY" => {
            let enabled = value.map(|v| v != "0").unwrap_or(true);
            Ok(SockOpt::TcpNoDelay(enabled))
        }
        "SO_REUSEADDR" => {
            let enabled = value.map(|v| v != "0").unwrap_or(true);
            Ok(SockOpt::ReuseAddr(enabled))
        }
        "SO_BINDTODEVICE" => {
            let v = value.ok_or_else(|| "SO_BINDTODEVICE requires a value".to_string())?;
            if v.is_empty() {
                return Err("SO_BINDTODEVICE requires a non-empty value".to_string());
            }
            Ok(SockOpt::BindToDevice(v.to_string()))
        }
        _ => Err(format!("unknown socket option: {name}")),
    }
}

fn parse_prefixed_sockopt(prefix: &str, rest: &str) -> Result<SockOpt, String> {
    match prefix.to_ascii_lowercase().as_str() {
        "ip" => {
            let (name, value) = rest
                .split_once('=')
                .ok_or_else(|| "ip option requires a value".to_string())?;
            let val = parse_u32(value)?;
            match name.to_ascii_lowercase().as_str() {
                "ttl" => Ok(SockOpt::IpTtl(val)),
                "tos" => Ok(SockOpt::IpTos(val)),
                "hoplimit" => Ok(SockOpt::IpHopLimit(val)),
                _ => Err(format!("unknown ip socket option: {name}")),
            }
        }
        _ => Err(format!("unknown socket option: {prefix}:{rest}")),
    }
}

fn parse_u32(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).map_err(|_| "invalid numeric value".to_string())
    } else {
        s.parse::<u32>()
            .map_err(|_| "invalid numeric value".to_string())
    }
}
