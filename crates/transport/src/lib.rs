// crates/transport/src/lib.rs
use std::io;
use std::time::Duration;

mod daemon;
mod factory;
mod rate;
#[cfg(unix)]
pub mod ssh;
mod stdio;
pub mod tcp;

pub use daemon::{DaemonTransport, SockOpt, daemon_remote_opts, parse_sockopts};
pub use factory::TransportFactory;
pub use rate::RateLimitedTransport;
#[cfg(unix)]
pub use ssh::SshStdioTransport;
pub use stdio::{LocalPipeTransport, TimeoutTransport};
pub use tcp::TcpTransport;

#[cfg(not(unix))]
use compress::Codec;

#[cfg(not(unix))]
pub struct SshStdioTransport;

#[cfg(not(unix))]
impl SshStdioTransport {
    pub fn spawn<I, S>(_program: &str, _args: I) -> io::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ssh transport is not supported on this platform",
        ))
    }

    pub fn spawn_server<I, S>(
        _host: &str,
        _server_args: I,
        _remote_opts: &[String],
        _known_hosts: Option<&std::path::Path>,
        _strict_host_key_checking: bool,
        _port: Option<u16>,
        _family: Option<AddressFamily>,
    ) -> io::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ssh transport is not supported on this platform",
        ))
    }

    pub fn spawn_from_command(_cmd: std::process::Command) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ssh transport is not supported on this platform",
        ))
    }

    pub fn handshake<T: Transport>(
        _transport: &mut T,
        _env: &[(String, String)],
        _remote_opts: &[String],
        _token: Option<&str>,
        _version: u32,
        _caps: u32,
    ) -> io::Result<(Vec<Codec>, u32)> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ssh transport is not supported on this platform",
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_with_rsh(
        _host: &str,
        _path: &std::path::Path,
        _rsh: &[String],
        _rsh_env: &[(String, String)],
        _remote_bin: Option<&[String]>,
        _remote_bin_env: &[(String, String)],
        _remote_opts: &[String],
        _known_hosts: Option<&std::path::Path>,
        _strict_host_key_checking: bool,
        _port: Option<u16>,
        _connect_timeout: Option<std::time::Duration>,
        _family: Option<AddressFamily>,
        _blocking_io: bool,
    ) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ssh transport is not supported on this platform",
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn connect_with_rsh(
        _host: &str,
        _path: &std::path::Path,
        _rsh: &[String],
        _rsh_env: &[(String, String)],
        _rsync_env: &[(String, String)],
        _remote_bin: Option<&[String]>,
        _remote_bin_env: &[(String, String)],
        _remote_opts: &[String],
        _known_hosts: Option<&std::path::Path>,
        _strict_host_key_checking: bool,
        _port: Option<u16>,
        _connect_timeout: Option<std::time::Duration>,
        _family: Option<AddressFamily>,
        _blocking_io: bool,
        _version: u32,
        _caps: u32,
        _token: Option<&str>,
    ) -> io::Result<(Self, Vec<Codec>, u32)> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ssh transport is not supported on this platform",
        ))
    }

    pub fn stderr(&self) -> (Vec<u8>, bool) {
        (Vec::new(), false)
    }

    pub fn into_inner(
        self,
    ) -> io::Result<(
        std::io::BufReader<std::process::ChildStdout>,
        std::process::ChildStdin,
    )> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ssh transport is not supported on this platform",
        ))
    }
}

#[cfg(not(unix))]
impl Transport for SshStdioTransport {
    fn send(&mut self, _data: &[u8]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ssh transport is not supported on this platform",
        ))
    }

    fn receive(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ssh transport is not supported on this platform",
        ))
    }

    fn close(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(not(unix))]
impl SshTransport for SshStdioTransport {}

pub fn rate_limited<T: Transport>(inner: T, bwlimit: u64) -> RateLimitedTransport<T> {
    RateLimitedTransport::new(inner, bwlimit)
}

pub fn pipe<S, D>(src: &mut S, dst: &mut D) -> io::Result<u64>
where
    S: Transport,
    D: Transport,
{
    let mut buf = [0u8; 8192];
    let mut total = 0u64;
    loop {
        let n = loop {
            match src.receive(&mut buf) {
                Ok(n) => break n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        };
        if n == 0 {
            break;
        }
        src.update_timeout();
        dst.update_timeout();
        loop {
            match dst.send(&buf[..n]) {
                Ok(()) => break,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                    src.update_timeout();
                    dst.update_timeout();
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        src.update_timeout();
        dst.update_timeout();
        total += n as u64;
    }
    Ok(total)
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

    fn close(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn update_timeout(&mut self) {}
}

pub trait SshTransport: Transport {}
