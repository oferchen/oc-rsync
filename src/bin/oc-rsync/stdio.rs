// src/bin/oc-rsync/stdio.rs
use oc_rsync_cli::options::OutBuf;
use std::fmt;
use std::io::{self, ErrorKind};
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicI32, Ordering};

#[cfg(not(target_os = "windows"))]
unsafe extern "C" {
    #[cfg_attr(target_os = "macos", link_name = "__stdoutp")]
    static mut stdout: *mut libc::FILE;
    #[cfg_attr(target_os = "macos", link_name = "__stderrp")]
    static mut stderr: *mut libc::FILE;
}

#[cfg(not(target_os = "windows"))]
fn stdout_stream() -> io::Result<NonNull<libc::FILE>> {
    // SAFETY: `stdout` is provided by the C runtime and assumed to be a valid pointer.
    unsafe {
        NonNull::new(stdout).ok_or_else(|| io::Error::new(ErrorKind::BrokenPipe, "stdout is null"))
    }
}

#[cfg(not(target_os = "windows"))]
fn stderr_stream() -> io::Result<NonNull<libc::FILE>> {
    // SAFETY: `stderr` is provided by the C runtime and assumed to be a valid pointer.
    unsafe {
        NonNull::new(stderr).ok_or_else(|| io::Error::new(ErrorKind::BrokenPipe, "stderr is null"))
    }
}

#[cfg(target_os = "windows")]
fn stdout_stream() -> io::Result<NonNull<libc::FILE>> {
    // SAFETY: `__acrt_iob_func` returns a valid pointer for stdout when called with index 1.
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
    // SAFETY: `__acrt_iob_func` returns a valid pointer for stderr when called with index 2.
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
    // SAFETY: `stream` was validated as non-null and the buffer arguments are well-formed.
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

fn set_std_buffering_raw_impl<F>(
    mode: libc::c_int,
    orig_mode: libc::c_int,
    out: *mut libc::FILE,
    err: *mut libc::FILE,
    set_stream: F,
) -> Result<(), StdBufferError>
where
    F: Fn(*mut libc::FILE, libc::c_int) -> io::Result<()>,
{
    let out_res = set_stream(out, mode);
    if out_res.is_err() && !out.is_null() {
        let _ = set_stream(out, orig_mode);
    }
    let err_res = set_stream(err, mode);
    if err_res.is_err() {
        if !err.is_null() {
            let _ = set_stream(err, orig_mode);
        }
        if out_res.is_ok() && !out.is_null() {
            let _ = set_stream(out, orig_mode);
        }
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

fn set_std_buffering_raw(
    mode: libc::c_int,
    orig_mode: libc::c_int,
    out: *mut libc::FILE,
    err: *mut libc::FILE,
) -> Result<(), StdBufferError> {
    set_std_buffering_raw_impl(mode, orig_mode, out, err, set_stream_buffer)
}

static STDOUT_MODE: AtomicI32 = AtomicI32::new(libc::_IOLBF);

pub fn set_std_buffering(mode: OutBuf) -> Result<(), StdBufferError> {
    let mode = match mode {
        OutBuf::N => libc::_IONBF,
        OutBuf::L => libc::_IOLBF,
        OutBuf::B => libc::_IOFBF,
    };
    let out = stdout_stream().map_err(StdBufferError::Stdout)?.as_ptr();
    let err = stderr_stream().map_err(StdBufferError::Stderr)?.as_ptr();
    let orig = STDOUT_MODE.load(Ordering::SeqCst);
    match set_std_buffering_raw(mode, orig, out, err) {
        Ok(()) => {
            STDOUT_MODE.store(mode, Ordering::SeqCst);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
pub(crate) fn set_std_buffering_for_test<F>(
    mode: libc::c_int,
    orig_mode: libc::c_int,
    out: *mut libc::FILE,
    err: *mut libc::FILE,
    set_stream: F,
) -> Result<(), StdBufferError>
where
    F: Fn(*mut libc::FILE, libc::c_int) -> io::Result<()>,
{
    set_std_buffering_raw_impl(mode, orig_mode, out, err, set_stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, io};

    #[test]
    fn ok_with_valid_streams() {
        assert!(set_std_buffering(OutBuf::L).is_ok());
    }

    #[test]
    fn stdout_failure() {
        let mut out_stub = std::mem::MaybeUninit::<libc::FILE>::uninit();
        let mut err_stub = std::mem::MaybeUninit::<libc::FILE>::uninit();
        let out: *mut libc::FILE = out_stub.as_mut_ptr();
        let err: *mut libc::FILE = err_stub.as_mut_ptr();
        let set_stream = |stream: *mut libc::FILE, _mode: libc::c_int| {
            if stream == out {
                Err(io::Error::other("stdout failure"))
            } else {
                Ok(())
            }
        };
        let res = set_std_buffering_for_test(libc::_IONBF, libc::_IOLBF, out, err, set_stream);
        assert!(matches!(res, Err(StdBufferError::Stdout(_))));
    }

    #[test]
    fn stderr_failure() {
        let mut out_stub = std::mem::MaybeUninit::<libc::FILE>::uninit();
        let mut err_stub = std::mem::MaybeUninit::<libc::FILE>::uninit();
        let out: *mut libc::FILE = out_stub.as_mut_ptr();
        let err: *mut libc::FILE = err_stub.as_mut_ptr();
        let set_stream = |stream: *mut libc::FILE, _mode: libc::c_int| {
            if stream == err {
                Err(io::Error::other("stderr failure"))
            } else {
                Ok(())
            }
        };
        let res = set_std_buffering_for_test(libc::_IONBF, libc::_IOLBF, out, err, set_stream);
        assert!(matches!(res, Err(StdBufferError::Stderr(_))));
    }

    #[test]
    fn stderr_failure_resets_streams() {
        let mut out_stub = std::mem::MaybeUninit::<libc::FILE>::uninit();
        let mut err_stub = std::mem::MaybeUninit::<libc::FILE>::uninit();
        let out: *mut libc::FILE = out_stub.as_mut_ptr();
        let err: *mut libc::FILE = err_stub.as_mut_ptr();
        let calls = RefCell::new(Vec::new());
        let set_stream = |stream: *mut libc::FILE, mode: libc::c_int| {
            calls.borrow_mut().push((stream, mode));
            if stream == err {
                Err(io::Error::other("stderr failure"))
            } else {
                Ok(())
            }
        };
        let res = set_std_buffering_for_test(libc::_IONBF, libc::_IOLBF, out, err, set_stream);
        assert!(matches!(res, Err(StdBufferError::Stderr(_))));
        let calls = calls.into_inner();
        assert_eq!(
            calls,
            vec![
                (out, libc::_IONBF),
                (err, libc::_IONBF),
                (err, libc::_IOLBF),
                (out, libc::_IOLBF),
            ]
        );
    }

    #[test]
    fn both_failure() {
        let out: *mut libc::FILE = std::ptr::dangling_mut();
        let err: *mut libc::FILE = std::ptr::dangling_mut();
        let set_stream =
            |_stream: *mut libc::FILE, _mode: libc::c_int| Err(io::Error::other("fail"));
        let res = set_std_buffering_for_test(libc::_IONBF, libc::_IOLBF, out, err, set_stream);
        assert!(matches!(res, Err(StdBufferError::Both { .. })));
    }
}
