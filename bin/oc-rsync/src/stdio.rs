// bin/oc-rsync/src/stdio.rs
use oc_rsync_cli::options::OutBuf;
use std::io::{self, ErrorKind};
use std::ptr::{self, NonNull};

#[cfg(unix)]
pub(crate) fn stdout_stream() -> io::Result<NonNull<libc::FILE>> {
    use std::os::unix::io::AsRawFd;
    let fd = io::stdout().as_raw_fd();
    let stream = unsafe { libc::fdopen(fd, c"w".as_ptr()) };
    NonNull::new(stream).ok_or_else(|| io::Error::new(ErrorKind::BrokenPipe, "stdout is null"))
}

#[cfg(windows)]
pub(crate) fn stdout_stream() -> io::Result<NonNull<libc::FILE>> {
    use std::os::windows::io::AsRawHandle;
    let handle = io::stdout().as_raw_handle();
    let fd = unsafe { libc::_open_osfhandle(handle as isize, 0) };
    if fd == -1 {
        return Err(io::Error::last_os_error());
    }
    let stream = unsafe { libc::_fdopen(fd, c"w".as_ptr()) };
    NonNull::new(stream).ok_or_else(|| io::Error::new(ErrorKind::BrokenPipe, "stdout is null"))
}

pub(crate) fn set_stream_buffer(stream: *mut libc::FILE, mode: libc::c_int) -> io::Result<()> {
    if stream.is_null() {
        return Err(io::Error::new(ErrorKind::BrokenPipe, "stream is null"));
    }
    let ret = unsafe { libc::setvbuf(stream, ptr::null_mut(), mode, 0) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub fn set_stdout_buffering(mode: OutBuf) -> io::Result<()> {
    let mode = match mode {
        OutBuf::N => libc::_IONBF,
        OutBuf::L => libc::_IOLBF,
        OutBuf::B => libc::_IOFBF,
    };
    let stream = stdout_stream()?;
    set_stream_buffer(stream.as_ptr(), mode)
}
