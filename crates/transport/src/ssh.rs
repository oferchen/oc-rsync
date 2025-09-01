// crates/transport/src/ssh.rs
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use std::ffi::OsStr;
use std::io::{self, BufReader, Read, Write};
use std::os::unix::io::{AsRawFd, BorrowedFd, RawFd};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use compress::{available_codecs, Codec};
use protocol::{negotiate_version, Frame, FrameHeader, Message, Msg, Tag, CAP_CODECS};

use crate::{AddressFamily, LocalPipeTransport, SshTransport, Transport};

const SSH_IO_BUF_SIZE: usize = 32 * 1024;
const SSH_STDERR_CAP: usize = 32 * 1024;

pub struct SshStdioTransport {
    inner: Option<LocalPipeTransport<BufReader<ChildStdout>, ChildStdin>>,
    stderr: Arc<Mutex<CapturedStderr>>,
    handle: Option<ProcessHandle>,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
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
        Self::spawn_from_command(cmd)
    }

    pub fn spawn_server<I, S>(
        host: &str,
        server_args: I,
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
        cmd.args(server_args);

        Self::spawn_from_command(cmd)
    }

    pub fn spawn_from_command(mut cmd: Command) -> io::Result<Self> {
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

        Ok(Self {
            inner: Some(LocalPipeTransport::new(
                BufReader::with_capacity(SSH_IO_BUF_SIZE, stdout),
                stdin,
            )),
            stderr: stderr_buf,
            handle: Some(handle),
            read_timeout: None,
            write_timeout: None,
        })
    }

    pub fn handshake<T: Transport>(
        transport: &mut T,
        env: &[(String, String)],
        remote_opts: &[String],
        version: u32,
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
        transport.send(&version.to_be_bytes())?;

        let mut ver_buf = [0u8; 4];
        let mut read = 0;
        while read < ver_buf.len() {
            let n = transport.receive(&mut ver_buf[read..])?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "failed to read version",
                ));
            }
            read += n;
        }
        let peer = u32::from_be_bytes(ver_buf);
        negotiate_version(version, peer).map_err(|e| io::Error::other(e.to_string()))?;

        let local_caps = CAP_CODECS;
        transport.send(&local_caps.to_be_bytes())?;

        let mut cap_buf = [0u8; 4];
        let mut read = 0;
        while read < cap_buf.len() {
            let n = transport.receive(&mut cap_buf[read..])?;
            if n == 0 {
                return Err(io::Error::other("failed to read capabilities"));
            }
            read += n;
        }
        let server_caps = u32::from_be_bytes(cap_buf);
        let caps = server_caps & local_caps;

        let mut peer_codecs = vec![Codec::Zlib];
        if caps & CAP_CODECS != 0 {
            let payload = compress::encode_codecs(&available_codecs());
            let frame = Message::Codecs(payload).to_frame(0);
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
                    return Err(io::Error::other("failed to read frame header"));
                }
                read += n;
            }
            let channel = u16::from_be_bytes([hdr[0], hdr[1]]);
            let tag = Tag::try_from(hdr[2]).map_err(io::Error::from)?;
            let msg = Msg::try_from(hdr[3]).map_err(io::Error::from)?;
            let len = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
            let mut payload = vec![0u8; len];
            let mut off = 0;
            while off < len {
                let n = transport.receive(&mut payload[off..])?;
                if n == 0 {
                    return Err(io::Error::other("failed to read frame payload"));
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
            let msg = Message::from_frame(frame).map_err(|e| io::Error::other(e.to_string()))?;
            if let Message::Codecs(data) = msg {
                peer_codecs =
                    compress::decode_codecs(&data).map_err(|e| io::Error::other(e.to_string()))?;
            }
        }

        Ok((peer_codecs, caps))
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
            cmd.args(remote_opts);
            cmd.arg("--server");
            cmd.arg(path.as_os_str());
            Self::spawn_from_command(cmd)
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
            args.extend_from_slice(remote_opts);
            args.push("--server".to_string());
            args.push(path.to_string_lossy().into_owned());
            let mut cmd = Command::new(program);
            cmd.args(args);
            cmd.envs(rsh_env.iter().cloned());
            Self::spawn_from_command(cmd)
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
        family: Option<AddressFamily>,
        version: u32,
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
        )?;
        if let Some(dur) = connect_timeout {
            let elapsed = start.elapsed();
            let remaining = dur
                .checked_sub(elapsed)
                .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "connection timed out"))?;
            t.set_read_timeout(Some(remaining))?;
            t.set_write_timeout(Some(remaining))?;
        }
        let (codecs, caps) = Self::handshake(&mut t, rsync_env, remote_opts, version)?;
        if connect_timeout.is_some() {
            t.set_read_timeout(None)?;
            t.set_write_timeout(None)?;
        }
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

fn wait_fd(fd: RawFd, flags: PollFlags, timeout: Option<Duration>) -> io::Result<()> {
    if let Some(dur) = timeout {
        let timeout =
            PollTimeout::try_from(dur).map_err(|_| io::Error::other("timeout overflow"))?;
        let mut fds = [PollFd::new(unsafe { BorrowedFd::borrow_raw(fd) }, flags)];
        let res = poll(&mut fds, timeout).map_err(io::Error::from)?;
        if res == 0 {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "operation timed out",
            ));
        }
    }
    Ok(())
}

impl Transport for SshStdioTransport {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        let pipe = inner_pipe(self.inner.as_mut())?;
        {
            let writer = pipe.writer_mut();
            let fd = writer.as_raw_fd();
            wait_fd(fd, PollFlags::POLLOUT, self.write_timeout)?;
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
        wait_fd(fd, PollFlags::POLLIN, self.read_timeout)?;
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
