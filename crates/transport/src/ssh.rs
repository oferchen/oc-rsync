use std::ffi::OsStr;
use std::io::{self, BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::{LocalPipeTransport, SshTransport, Transport};

const SSH_IO_BUF_SIZE: usize = 32 * 1024;
const SSH_STDERR_CAP: usize = 32 * 1024;

/// Transport over the stdio of a spawned `ssh` process.
pub struct SshStdioTransport {
    inner: LocalPipeTransport<BufReader<ChildStdout>, BufWriter<ChildStdin>>,
    stderr: Arc<Mutex<CapturedStderr>>,
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

    fn spawn_from_command(mut cmd: Command) -> io::Result<Self> {
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
        std::thread::spawn(move || {
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

        Ok(Self {
            inner: LocalPipeTransport::new(
                BufReader::with_capacity(SSH_IO_BUF_SIZE, stdout),
                BufWriter::with_capacity(SSH_IO_BUF_SIZE, stdin),
            ),
            stderr: stderr_buf,
        })
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

    /// Consume the transport returning the buffered reader and writer.
    pub fn into_inner(self) -> (BufReader<ChildStdout>, BufWriter<ChildStdin>) {
        self.inner.into_inner()
    }
}

impl Transport for SshStdioTransport {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.inner.send(data)
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.receive(buf)
    }
}

impl SshTransport for SshStdioTransport {}
