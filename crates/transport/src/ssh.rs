// crates/transport/src/ssh.rs

use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use std::ffi::OsStr;
use std::io::{self, BufReader, Read, Write};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use checksums::{strong_digest, StrongHash};
use compress::{available_codecs, Codec};
use protocol::{negotiate_version, Frame, FrameHeader, Message, Msg, Tag, CAP_CODECS};

use crate::{AddressFamily, LocalPipeTransport, SshTransport, Transport};

const SSH_IO_BUF_SIZE: usize = 32 * 1024;
const SSH_STDERR_CAP: usize = 32 * 1024;
pub const MAX_FRAME_LEN: usize = 16 * 1024 * 1024;

pub struct SshStdioTransport {
    inner: Option<LocalPipeTransport<BufReader<ChildStdout>, ChildStdin>>,
    stderr: Arc<Mutex<CapturedStderr>>,
    handle: Option<ProcessHandle>,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
    blocking_io: bool,
}

struct ProcessHandle {
    child: Child,
    stderr_thread: Option<JoinHandle<()>>,
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        if let Some(handle) = self.stderr_thread.take() {
            let _ = handle.join();
        }
        let _ = self.child.wait();
    }
}

#[derive(Default)]
struct CapturedStderr {
    data: Vec<u8>,
    truncated: bool,
}

impl SshStdioTransport {
    pub fn spawn<I, S>(program: &str, args: I) -> io::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new(program);
        cmd.args(args);
        Self::spawn_from_command(cmd, false)
    }

    pub fn spawn_server<I, S>(
        host: &str,
        server_args: I,
        remote_opts: &[String],
        known_hosts: Option<&Path>,
        strict_host_key_checking: bool,
        port: Option<u16>,
        family: Option<AddressFamily>,
    ) -> io::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new("ssh");

        let known_hosts_path = known_hosts.map(Path::to_path_buf).or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".ssh/known_hosts"))
        });

        let checking = if strict_host_key_checking {
            "yes"
        } else {
            "no"
        };
        cmd.arg("-o")
            .arg(format!("StrictHostKeyChecking={checking}"));
        if let Some(path) = known_hosts_path {
            cmd.arg("-o")
                .arg(format!("UserKnownHostsFile={}", path.display()));
        }
        if let Some(p) = port {
            cmd.arg("-p").arg(p.to_string());
        }
        if let Some(AddressFamily::V4) = family {
            cmd.arg("-4");
        } else if let Some(AddressFamily::V6) = family {
            cmd.arg("-6");
        }
        cmd.arg(host);
        cmd.arg("rsync");
        cmd.arg("--server");
        cmd.args(remote_opts);
        cmd.args(server_args);

        Self::spawn_from_command(cmd, false)
    }

    pub fn spawn_from_command(mut cmd: Command, blocking_io: bool) -> io::Result<Self> {
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("missing stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("missing stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("missing stderr"))?;

        let stderr_buf = Arc::new(Mutex::new(CapturedStderr::default()));
        let stderr_buf_clone = Arc::clone(&stderr_buf);
        let stderr_thread = std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut buf = Vec::new();
            let mut chunk = [0u8; SSH_IO_BUF_SIZE];
            let mut truncated = false;
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        if buf.len() < SSH_STDERR_CAP {
                            let remaining = SSH_STDERR_CAP - buf.len();
                            let take = remaining.min(n);
                            buf.extend_from_slice(&chunk[..take]);
                            if n > take {
                                truncated = true;
                            }
                        } else {
                            truncated = true;
                        }
                    }
                    Err(_) => break,
                }
            }
            if let Ok(mut locked) = stderr_buf_clone.lock() {
                locked.data = buf;
                locked.truncated = truncated;
            }
        });

        let handle = ProcessHandle {
            child,
            stderr_thread: Some(stderr_thread),
        };

        let mut t = Self {
            inner: Some(LocalPipeTransport::new(
                BufReader::with_capacity(SSH_IO_BUF_SIZE, stdout),
                stdin,
            )),
            stderr: stderr_buf,
            handle: Some(handle),
            read_timeout: None,
            write_timeout: None,
            blocking_io: false,
        };
        t.set_blocking_io(blocking_io)?;
        Ok(t)
    }

    pub fn set_blocking_io(&mut self, blocking: bool) -> io::Result<()> {
        if let Some(pipe) = self.inner.as_mut() {
            let reader_fd = pipe.reader_mut().get_ref().as_raw_fd();
            let writer_fd = pipe.writer_mut().as_raw_fd();
            set_fd_blocking(reader_fd, blocking)?;
            set_fd_blocking(writer_fd, blocking)?;
        }
        self.blocking_io = blocking;
        Ok(())
    }

    pub fn handshake<T: Transport>(
        transport: &mut T,
        env: &[(String, String)],
        remote_opts: &[String],
        token: Option<&str>,
        version: u32,
        caps: u32,
    ) -> io::Result<(Vec<Codec>, u32)> {
        for opt in remote_opts {
            let mut buf = Vec::new();
            buf.extend_from_slice(opt.as_bytes());
            buf.push(0);
            transport.send(&buf)?;
        }
        for (k, v) in env {
            let mut buf = Vec::new();
            buf.extend_from_slice(k.as_bytes());
            buf.push(b'=');
            buf.extend_from_slice(v.as_bytes());
            buf.push(0);
            transport.send(&buf)?;
        }
        transport.send(&[0])?;
        if let Some(tok) = token {
            let mut challenge = [0u8; 16];
            let mut read = 0;
            while read < challenge.len() {
                let n = transport.receive(&mut challenge[read..])?;
                if n == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "connection unexpectedly closed",
                    ));
                }
                read += n;
            }
            let mut data = challenge.to_vec();
            data.extend_from_slice(tok.as_bytes());
            let resp = strong_digest(&data, StrongHash::Md5, 0);
            transport.send(&resp)?;
        }
        transport.send(&version.to_be_bytes())?;

        let mut ver_buf = [0u8; 4];
        let mut read = 0;
        while read < ver_buf.len() {
            let n = transport.receive(&mut ver_buf[read..])?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "connection unexpectedly closed",
                ));
            }
            read += n;
        }
        let peer = u32::from_be_bytes(ver_buf);
        negotiate_version(version, peer).map_err(|e| io::Error::other(e.to_string()))?;

        let local_caps = caps | CAP_CODECS;
        transport.send(&local_caps.to_be_bytes())?;

        let mut cap_buf = [0u8; 4];
        let mut read = 0;
        while read < cap_buf.len() {
            let n = transport.receive(&mut cap_buf[read..])?;
            if n == 0 {
                if read == 0 {
                    break;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "connection unexpectedly closed",
                    ));
                }
            }
            read += n;
        }
        let server_caps = if read == 4 {
            u32::from_be_bytes(cap_buf)
        } else {
            0
        };
        let common_caps = server_caps & local_caps;

        let mut peer_codecs = vec![Codec::Zlib];
        if common_caps & CAP_CODECS != 0 {
            let payload = compress::encode_codecs(&available_codecs());
            let frame = Message::Codecs(payload).to_frame(0, None);
            let mut buf = Vec::new();
            frame
                .encode(&mut buf)
                .map_err(|e| io::Error::other(e.to_string()))?;
            transport.send(&buf)?;

            let mut hdr = [0u8; 8];
            let mut read = 0;
            while read < hdr.len() {
                let n = transport.receive(&mut hdr[read..])?;
                if n == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "connection unexpectedly closed",
                    ));
                }
                read += n;
            }
            let channel = u16::from_be_bytes([hdr[0], hdr[1]]);
            let tag = Tag::try_from(hdr[2]).map_err(io::Error::from)?;
            let msg = Msg::try_from(hdr[3]).map_err(io::Error::from)?;
            let len = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
            if len > MAX_FRAME_LEN {
                return Err(io::Error::other("frame length exceeds maximum"));
            }
            let mut payload = vec![0u8; len];
            let mut off = 0;
            while off < len {
                let n = transport.receive(&mut payload[off..])?;
                if n == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "connection unexpectedly closed",
                    ));
                }
                off += n;
            }
            let frame = Frame {
                header: FrameHeader {
                    channel,
                    tag,
                    msg,
                    len: len as u32,
                },
                payload,
            };
            let msg =
                Message::from_frame(frame, None).map_err(|e| io::Error::other(e.to_string()))?;
            if let Message::Codecs(data) = msg {
                peer_codecs =
                    compress::decode_codecs(&data).map_err(|e| io::Error::other(e.to_string()))?;
            }
        }

        Ok((peer_codecs, common_caps))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_with_rsh(
        host: &str,
        path: &Path,
        rsh: &[String],
        rsh_env: &[(String, String)],
        remote_bin: Option<&[String]>,
        remote_bin_env: &[(String, String)],
        remote_opts: &[String],
        known_hosts: Option<&Path>,
        strict_host_key_checking: bool,
        port: Option<u16>,
        connect_timeout: Option<Duration>,
        family: Option<AddressFamily>,
        blocking_io: bool,
    ) -> io::Result<Self> {
        let program = rsh.first().map(|s| s.as_str()).unwrap_or("ssh");
        if program == "ssh" {
            let mut cmd = Command::new(program);
            cmd.args(&rsh[1..]);
            cmd.envs(rsh_env.iter().cloned());
            let known_hosts_path = known_hosts.map(Path::to_path_buf).or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".ssh/known_hosts"))
            });
            let checking = if strict_host_key_checking {
                "yes"
            } else {
                "no"
            };
            cmd.arg("-o")
                .arg(format!("StrictHostKeyChecking={checking}"));
            if let Some(path) = known_hosts_path {
                cmd.arg("-o")
                    .arg(format!("UserKnownHostsFile={}", path.display()));
            }
            if let Some(dur) = connect_timeout {
                cmd.arg("-o")
                    .arg(format!("ConnectTimeout={}", dur.as_secs()));
            }
            if let Some(p) = port {
                cmd.arg("-p").arg(p.to_string());
            }
            if let Some(AddressFamily::V4) = family {
                cmd.arg("-4");
            } else if let Some(AddressFamily::V6) = family {
                cmd.arg("-6");
            }
            cmd.arg(host);
            if let Some(bin) = remote_bin {
                for (k, v) in remote_bin_env {
                    cmd.arg(format!("{k}={v}"));
                }
                cmd.args(bin);
            } else {
                cmd.arg("rsync");
            }
            cmd.arg("--server");
            cmd.args(remote_opts);
            cmd.arg(path.as_os_str());
            Self::spawn_from_command(cmd, blocking_io)
        } else {
            let mut args = rsh[1..].to_vec();
            let host = if let Some(p) = port {
                format!("{host}:{p}")
            } else {
                host.to_string()
            };
            args.push(host);
            if let Some(bin) = remote_bin {
                for (k, v) in remote_bin_env {
                    args.push(format!("{k}={v}"));
                }
                args.extend_from_slice(bin);
            } else {
                args.push("rsync".to_string());
            }
            args.push("--server".to_string());
            args.extend_from_slice(remote_opts);
            args.push(path.to_string_lossy().into_owned());
            let mut cmd = Command::new(program);
            cmd.args(args);
            cmd.envs(rsh_env.iter().cloned());
            Self::spawn_from_command(cmd, blocking_io)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn connect_with_rsh(
        host: &str,
        path: &Path,
        rsh: &[String],
        rsh_env: &[(String, String)],
        rsync_env: &[(String, String)],
        remote_bin: Option<&[String]>,
        remote_bin_env: &[(String, String)],
        remote_opts: &[String],
        known_hosts: Option<&Path>,
        strict_host_key_checking: bool,
        port: Option<u16>,
        connect_timeout: Option<Duration>,
        timeout: Option<Duration>,
        family: Option<AddressFamily>,
        blocking_io: bool,
        version: u32,
        caps: u32,
        token: Option<&str>,
    ) -> io::Result<(Self, Vec<Codec>, u32)> {
        let start = Instant::now();
        let mut t = Self::spawn_with_rsh(
            host,
            path,
            rsh,
            rsh_env,
            remote_bin,
            remote_bin_env,
            remote_opts,
            known_hosts,
            strict_host_key_checking,
            port,
            connect_timeout,
            family,
            blocking_io,
        )?;
        if let Some(dur) = connect_timeout {
            let elapsed = start.elapsed();
            let remaining = dur
                .checked_sub(elapsed)
                .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "connection timed out"))?;
            t.set_read_timeout(Some(remaining))?;
            t.set_write_timeout(Some(remaining))?;
        } else {
            t.set_read_timeout(timeout)?;
            t.set_write_timeout(timeout)?;
        }
        let (codecs, caps) =
            match Self::handshake(&mut t, rsync_env, remote_opts, token, version, caps) {
                Ok(v) => v,
                Err(mut e) => {
                    let (stderr, _) = t.stderr();
                    if !stderr.is_empty() {
                        let mut msg = String::from_utf8_lossy(&stderr).into_owned();
                        if !msg.ends_with('\n') {
                            msg.push('\n');
                        }
                        msg.push_str("connection unexpectedly closed");
                        e = io::Error::new(io::ErrorKind::UnexpectedEof, msg);
                    }
                    return Err(e);
                }
            };
        t.set_read_timeout(timeout)?;
        t.set_write_timeout(timeout)?;
        Ok((t, codecs, caps))
    }

    pub fn stderr(&self) -> (Vec<u8>, bool) {
        if let Ok(buf) = self.stderr.lock() {
            (buf.data.clone(), buf.truncated)
        } else {
            (Vec::new(), false)
        }
    }

    pub fn into_inner(mut self) -> io::Result<(BufReader<ChildStdout>, ChildStdin)> {
        if let Some(handle) = self.handle.take() {
            std::mem::forget(handle);
        }
        let inner = self
            .inner
            .take()
            .ok_or_else(|| io::Error::other("missing inner transport"))?;
        Ok(inner.into_inner())
    }
}

type InnerPipe = LocalPipeTransport<BufReader<ChildStdout>, ChildStdin>;

fn inner_pipe(inner: Option<&mut InnerPipe>) -> io::Result<&mut InnerPipe> {
    inner.ok_or_else(|| io::Error::other("missing inner transport"))
}

fn set_fd_blocking(fd: RawFd, blocking: bool) -> io::Result<()> {
    use std::os::fd::BorrowedFd;
    let fd = unsafe { BorrowedFd::borrow_raw(fd) };
    let flags = OFlag::from_bits_truncate(fcntl(fd, FcntlArg::F_GETFL).map_err(io::Error::from)?);
    let mut new_flags = flags;
    if blocking {
        new_flags.remove(OFlag::O_NONBLOCK);
    } else {
        new_flags.insert(OFlag::O_NONBLOCK);
    }
    fcntl(fd, FcntlArg::F_SETFL(new_flags)).map_err(io::Error::from)?;
    Ok(())
}

fn wait_fd(fd: RawFd, flags: PollFlags, timeout: Option<Duration>) -> io::Result<()> {
    let timeout = match timeout {
        Some(dur) => {
            PollTimeout::try_from(dur).map_err(|_| io::Error::other("timeout overflow"))?
        }
        None => PollTimeout::NONE,
    };
    let mut fds = [PollFd::new(unsafe { BorrowedFd::borrow_raw(fd) }, flags)];
    let res = poll(&mut fds, timeout).map_err(io::Error::from)?;
    if res == 0 {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "operation timed out",
        ));
    }
    Ok(())
}

impl Transport for SshStdioTransport {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        let pipe = inner_pipe(self.inner.as_mut())?;
        {
            let writer = pipe.writer_mut();
            let fd = writer.as_raw_fd();
            if !self.blocking_io {
                wait_fd(fd, PollFlags::POLLOUT, self.write_timeout)?;
            }
            if let Err(err) = writer.write_all(data).and_then(|_| writer.flush()) {
                let (stderr, _) = self.stderr();
                if !stderr.is_empty() {
                    return Err(io::Error::new(
                        err.kind(),
                        String::from_utf8_lossy(&stderr).into_owned(),
                    ));
                }
                return Err(err);
            }
        }
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let pipe = inner_pipe(self.inner.as_mut())?;
        let reader = pipe.reader_mut();
        let fd = reader.get_ref().as_raw_fd();
        if !self.blocking_io {
            wait_fd(fd, PollFlags::POLLIN, self.read_timeout)?;
        }
        match reader.read(buf) {
            Ok(n) => Ok(n),
            Err(err) => {
                let (stderr, _) = self.stderr();
                if !stderr.is_empty() {
                    return Err(io::Error::new(
                        err.kind(),
                        String::from_utf8_lossy(&stderr).into_owned(),
                    ));
                }
                Err(err)
            }
        }
    }

    fn set_read_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
        self.read_timeout = dur;
        Ok(())
    }

    fn set_write_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
        self.write_timeout = dur;
        Ok(())
    }

    fn close(&mut self) -> io::Result<()> {
        let pipe = inner_pipe(self.inner.as_mut())?;
        pipe.writer_mut().flush()
    }
}

impl SshTransport for SshStdioTransport {}

impl Drop for SshStdioTransport {
    fn drop(&mut self) {
        self.inner.take();
        if let Some(handle) = self.handle.take() {
            drop(handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> SshStdioTransport {
        SshStdioTransport {
            inner: None,
            stderr: Arc::new(Mutex::new(CapturedStderr::default())),
            handle: None,
            read_timeout: None,
            write_timeout: None,
            blocking_io: false,
        }
    }

    #[test]
    fn send_fails_without_inner() {
        let mut t = empty();
        assert!(t.send(b"data").is_err());
    }

    #[test]
    fn receive_fails_without_inner() {
        let mut t = empty();
        assert!(t.receive(&mut [0u8; 1]).is_err());
    }

    #[test]
    fn into_inner_fails_without_inner() {
        let t = empty();
        assert!(t.into_inner().is_err());
    }
}
