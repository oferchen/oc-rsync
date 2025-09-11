// crates/transport/src/ssh/session.rs

use std::io::{self, BufReader};
use std::os::fd::AsRawFd;
use std::process::{Child, ChildStdin, ChildStdout};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use checksums::{StrongHash, strong_digest};
use compress::{Codec, available_codecs};
use protocol::{CAP_CODECS, Frame, FrameHeader, Message, Msg, Tag, negotiate_version};

use crate::{LocalPipeTransport, SshTransport, Transport};

pub(crate) const SSH_IO_BUF_SIZE: usize = 32 * 1024;
pub(crate) const SSH_STDERR_CAP: usize = 32 * 1024;
pub const MAX_FRAME_LEN: usize = 16 * 1024 * 1024;

pub struct SshStdioTransport {
    pub(crate) inner: Option<LocalPipeTransport<BufReader<ChildStdout>, ChildStdin>>,
    pub(crate) stderr: Arc<Mutex<CapturedStderr>>,
    pub(crate) handle: Option<ProcessHandle>,
    pub(crate) read_timeout: Option<Duration>,
    pub(crate) write_timeout: Option<Duration>,
    pub(crate) blocking_io: bool,
}

pub(crate) struct ProcessHandle {
    pub(crate) child: Child,
    pub(crate) stderr_thread: Option<JoinHandle<()>>,
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
pub(crate) struct CapturedStderr {
    pub(crate) data: Vec<u8>,
    pub(crate) truncated: bool,
}

impl SshStdioTransport {
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

    pub fn set_blocking_io(&mut self, blocking: bool) -> io::Result<()> {
        if let Some(pipe) = self.inner.as_mut() {
            let reader_fd = pipe.reader_mut().get_ref().as_raw_fd();
            let writer_fd = pipe.writer_mut().as_raw_fd();
            super::io::set_fd_blocking(reader_fd, blocking)?;
            super::io::set_fd_blocking(writer_fd, blocking)?;
        }
        self.blocking_io = blocking;
        Ok(())
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
