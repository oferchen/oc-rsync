use std::ffi::OsStr;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use compress::{available_codecs, Codec};
use protocol::{
    negotiate_version, Frame, FrameHeader, Message, Msg, Tag, CAP_CODECS, LATEST_VERSION,
};

use crate::{LocalPipeTransport, SshTransport, Transport};

const SSH_IO_BUF_SIZE: usize = 32 * 1024;
const SSH_STDERR_CAP: usize = 32 * 1024;

/// Transport over the stdio of a spawned `ssh` process.
pub struct SshStdioTransport {
    inner: Option<LocalPipeTransport<BufReader<ChildStdout>, ChildStdin>>,
    stderr: Arc<Mutex<CapturedStderr>>,
    handle: Option<ProcessHandle>,
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
    /// Spawn an SSH-like command and return a transport over its stdio.
    pub fn spawn<I, S>(program: &str, args: I) -> io::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new(program);
        cmd.args(args);
        Self::spawn_from_command(cmd)
    }

    /// Spawn a real `ssh` process targeting an rsync server and capture stderr.
    pub fn spawn_server<I, S>(
        host: &str,
        server_args: I,
        known_hosts: Option<&Path>,
        strict_host_key_checking: bool,
    ) -> io::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new("ssh");

        // Determine the known hosts file. Use the provided path or default to
        // the user's `~/.ssh/known_hosts` if available.
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

        cmd.arg(host);
        cmd.arg("rsync");
        cmd.arg("--server");
        cmd.args(server_args);

        Self::spawn_from_command(cmd)
    }

    /// Spawn from a fully configured command.
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
        })
    }

    fn handshake<T: Transport>(transport: &mut T) -> io::Result<Vec<Codec>> {
        transport.send(&LATEST_VERSION.to_be_bytes())?;

        let mut ver_buf = [0u8; 4];
        let mut read = 0;
        while read < ver_buf.len() {
            let n = transport.receive(&mut ver_buf[read..])?;
            if n == 0 {
                return Err(io::Error::other("failed to read version"));
            }
            read += n;
        }
        let peer = u32::from_be_bytes(ver_buf);
        negotiate_version(peer).map_err(|e| io::Error::other(e.to_string()))?;

        let caps = CAP_CODECS;
        transport.send(&caps.to_be_bytes())?;

        let mut cap_buf = [0u8; 4];
        transport.receive(&mut cap_buf)?;
        let server_caps = u32::from_be_bytes(cap_buf);

        let mut peer_codecs = vec![Codec::Zlib];
        if server_caps & CAP_CODECS != 0 {
            let payload = compress::encode_codecs(available_codecs());
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

        Ok(peer_codecs)
    }

    pub fn spawn_with_rsh(
        host: &str,
        path: &Path,
        rsh: &[String],
        remote_bin: Option<&Path>,
        known_hosts: Option<&Path>,
        strict_host_key_checking: bool,
    ) -> io::Result<Self> {
        let program = rsh.get(0).map(|s| s.as_str()).unwrap_or("ssh");
        if program == "ssh" {
            let mut cmd = Command::new(program);
            cmd.args(&rsh[1..]);
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
            cmd.arg(host);
            if let Some(bin) = remote_bin {
                cmd.arg(bin);
            } else {
                cmd.arg("rsync");
            }
            cmd.arg("--server");
            cmd.arg(path.as_os_str());
            Self::spawn_from_command(cmd)
        } else {
            let mut args = rsh[1..].to_vec();
            args.push(host.to_string());
            if let Some(bin) = remote_bin {
                args.push(bin.to_string_lossy().into_owned());
            } else {
                args.push("rsync".to_string());
            }
            args.push("--server".to_string());
            args.push(path.to_string_lossy().into_owned());
            Self::spawn(program, args)
        }
    }

    pub fn connect_with_rsh(
        host: &str,
        path: &Path,
        rsh: &[String],
        remote_bin: Option<&Path>,
        known_hosts: Option<&Path>,
        strict_host_key_checking: bool,
    ) -> io::Result<(Self, Vec<Codec>)> {
        let mut t = Self::spawn_with_rsh(
            host,
            path,
            rsh,
            remote_bin,
            known_hosts,
            strict_host_key_checking,
        )?;
        let codecs = Self::handshake(&mut t)?;
        Ok((t, codecs))
    }

    /// Return any data captured from stderr of the spawned process along with
    /// a flag indicating if the data was truncated.
    pub fn stderr(&self) -> (Vec<u8>, bool) {
        if let Ok(buf) = self.stderr.lock() {
            (buf.data.clone(), buf.truncated)
        } else {
            (Vec::new(), false)
        }
    }

    /// Consume the transport returning the reader and writer.
    ///
    /// This detaches the child process and stderr thread; the caller is
    /// responsible for reaping the process.
    pub fn into_inner(mut self) -> (BufReader<ChildStdout>, ChildStdin) {
        if let Some(handle) = self.handle.take() {
            std::mem::forget(handle);
        }
        self.inner.take().expect("inner").into_inner()
    }
}

impl Transport for SshStdioTransport {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        match self.inner.as_mut().expect("inner").send(data) {
            Ok(()) => Ok(()),
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

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.inner.as_mut().expect("inner").receive(buf) {
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
}

impl SshTransport for SshStdioTransport {}

impl Drop for SshStdioTransport {
    fn drop(&mut self) {
        // Close stdin/stdout before waiting on the child process.
        self.inner.take();
        if let Some(handle) = self.handle.take() {
            drop(handle);
        }
    }
}
