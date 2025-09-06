// src/bin/oc-rsync/stdio.rs
use oc_rsync_cli::options::OutBuf;
use std::fmt;
use std::io::{self, ErrorKind};
use std::mem::MaybeUninit;
use std::ptr::{self, NonNull};

#[cfg(not(target_os = "windows"))]
unsafe extern "C" {
    #[cfg_attr(target_os = "macos", link_name = "__stdoutp")]
    static mut stdout: *mut libc::FILE;
    #[cfg_attr(target_os = "macos", link_name = "__stderrp")]
    static mut stderr: *mut libc::FILE;
}

#[cfg(not(target_os = "windows"))]
fn stdout_stream() -> io::Result<NonNull<libc::FILE>> {
    unsafe {
        NonNull::new(stdout).ok_or_else(|| io::Error::new(ErrorKind::BrokenPipe, "stdout is null"))
    }
}

#[cfg(not(target_os = "windows"))]
fn stderr_stream() -> io::Result<NonNull<libc::FILE>> {
    unsafe {
        NonNull::new(stderr).ok_or_else(|| io::Error::new(ErrorKind::BrokenPipe, "stderr is null"))
    }
}

#[cfg(target_os = "windows")]
fn stdout_stream() -> io::Result<NonNull<libc::FILE>> {
    unsafe {
        extern "C" {
            fn __acrt_iob_func(idx: libc::c_uint) -> *mut libc::FILE;
        }
        NonNull::new(__acrt_iob_func(1))
            .ok_or_else(|| io::Error::new(ErrorKind::BrokenPipe, "__acrt_iob_func returned null"))
    }
}

#[cfg(target_os = "windows")]
fn stderr_stream() -> io::Result<NonNull<libc::FILE>> {
    unsafe {
        extern "C" {
            fn __acrt_iob_func(idx: libc::c_uint) -> *mut libc::FILE;
        }
        NonNull::new(__acrt_iob_func(2))
            .ok_or_else(|| io::Error::new(ErrorKind::BrokenPipe, "__acrt_iob_func returned null"))
    }
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

#[derive(Debug)]
pub enum StdBufferError {
    Stdout(io::Error),
    Stderr(io::Error),
    Both {
        stdout: io::Error,
        stderr: io::Error,
    },
}

impl fmt::Display for StdBufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StdBufferError::Stdout(e) => write!(f, "stdout: {e}"),
            StdBufferError::Stderr(e) => write!(f, "stderr: {e}"),
            StdBufferError::Both {
                stdout: s,
                stderr: e,
            } => {
                write!(f, "stdout: {s}, stderr: {e}")
            }
        }
    }
}

impl std::error::Error for StdBufferError {}

fn set_std_buffering_raw(
    mode: libc::c_int,
    out: *mut libc::FILE,
    err: *mut libc::FILE,
) -> Result<(), StdBufferError> {
    unsafe {
        let mut out_backup = MaybeUninit::<libc::FILE>::uninit();
        if !out.is_null() {
            ptr::copy_nonoverlapping(out, out_backup.as_mut_ptr(), 1);
        }
        let out_res = set_stream_buffer(out, mode);
        let err_res = set_stream_buffer(err, mode);
        if (out_res.is_err() || err_res.is_err()) && !out.is_null() {
            ptr::copy_nonoverlapping(out_backup.as_ptr(), out, 1);
        }
        match (out_res, err_res) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(o), Ok(())) => Err(StdBufferError::Stdout(o)),
            (Ok(()), Err(e)) => Err(StdBufferError::Stderr(e)),
            (Err(o), Err(e)) => Err(StdBufferError::Both {
                stdout: o,
                stderr: e,
            }),
        }
    }
}

pub fn set_std_buffering(mode: OutBuf) -> Result<(), StdBufferError> {
    let mode = match mode {
        OutBuf::N => libc::_IONBF,
        OutBuf::L => libc::_IOLBF,
        OutBuf::B => libc::_IOFBF,
    };
    let out = stdout_stream().map_err(StdBufferError::Stdout)?.as_ptr();
    let err = stderr_stream().map_err(StdBufferError::Stderr)?.as_ptr();
    set_std_buffering_raw(mode, out, err)
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) unsafe fn set_std_buffering_for_test(
    mode: libc::c_int,
    out: *mut libc::FILE,
    err: *mut libc::FILE,
) -> Result<(), StdBufferError> {
    set_std_buffering_raw(mode, out, err)
}
