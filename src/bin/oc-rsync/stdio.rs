// src/bin/oc-rsync/stdio.rs
use oc_rsync_cli::options::OutBuf;
use std::io::{self, ErrorKind};
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

pub fn set_std_buffering(mode: OutBuf) -> io::Result<()> {
    let mode = match mode {
        OutBuf::N => libc::_IONBF,
        OutBuf::L => libc::_IOLBF,
        OutBuf::B => libc::_IOFBF,
    };
    let out = stdout_stream()?;
    set_stream_buffer(out.as_ptr(), mode)?;
    let err = stderr_stream()?;
    set_stream_buffer(err.as_ptr(), mode)
}
