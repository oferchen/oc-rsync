// crates/transport/src/ssh/io.rs

use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::time::Duration;

use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};

use crate::LocalPipeTransport;
use crate::Transport;

use super::session::SshStdioTransport;
use std::io::BufReader;
use std::process::{ChildStdin, ChildStdout};

pub(crate) type InnerPipe = LocalPipeTransport<BufReader<ChildStdout>, ChildStdin>;

pub(crate) fn inner_pipe(inner: Option<&mut InnerPipe>) -> io::Result<&mut InnerPipe> {
    inner.ok_or_else(|| io::Error::other("missing inner transport"))
}

/// Borrow a raw file descriptor for the duration of an operation.
///
/// # Safety
/// `fd` must reference a valid, open file descriptor that remains so for the
/// lifetime of the returned [`BorrowedFd`].
unsafe fn borrow_fd(fd: RawFd) -> BorrowedFd<'static> {
    // SAFETY: caller guarantees that `fd` is a valid open file descriptor.
    unsafe { BorrowedFd::borrow_raw(fd) }
}

pub(crate) fn set_fd_blocking(fd: RawFd, blocking: bool) -> io::Result<()> {
    // SAFETY: `fd` must reference a valid open descriptor per `borrow_fd`'s contract.
    let fd = unsafe { borrow_fd(fd) };
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
    // SAFETY: `fd` must remain valid for the duration of the poll.
    let mut fds = [PollFd::new(unsafe { borrow_fd(fd) }, flags)];
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
