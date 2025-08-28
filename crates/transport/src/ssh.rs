use std::ffi::OsStr;
use std::io;
use std::process::{ChildStdin, ChildStdout, Command, Stdio};

use crate::{LocalPipeTransport, SshTransport, Transport};

/// Transport over the stdio of a spawned `ssh` process.
pub struct SshStdioTransport {
    inner: LocalPipeTransport<ChildStdout, ChildStdin>,
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
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
        let mut child = cmd.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing stdout"))?;
        Ok(Self {
            inner: LocalPipeTransport::new(stdout, stdin),
        })
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
