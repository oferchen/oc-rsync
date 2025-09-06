// tests/bin_stdio.rs
#[path = "../src/bin/oc-rsync/stdio.rs"]
mod stdio;

use oc_rsync_cli::options::OutBuf;
use std::mem;
use std::ptr;
use stdio::{StdBufferError, set_std_buffering, set_std_buffering_for_test, set_stream_buffer};

#[cfg(not(target_os = "windows"))]
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn stdout_ptr() -> *mut libc::FILE {
    unsafe extern "C" {
        #[cfg_attr(target_os = "macos", link_name = "__stdoutp")]
        static mut stdout: *mut libc::FILE;
    }
    unsafe { stdout }
}

#[cfg(not(target_os = "windows"))]
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn stderr_ptr() -> *mut libc::FILE {
    unsafe extern "C" {
        #[cfg_attr(target_os = "macos", link_name = "__stderrp")]
        static mut stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}

#[cfg(target_os = "windows")]
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn stdout_ptr() -> *mut libc::FILE {
    unsafe extern "C" {
        fn __acrt_iob_func(idx: libc::c_uint) -> *mut libc::FILE;
    }
    unsafe { __acrt_iob_func(1) }
}

#[cfg(target_os = "windows")]
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn stderr_ptr() -> *mut libc::FILE {
    unsafe extern "C" {
        fn __acrt_iob_func(idx: libc::c_uint) -> *mut libc::FILE;
    }
    unsafe { __acrt_iob_func(2) }
}

#[test]
fn mode_changes_ok() {
    set_std_buffering(OutBuf::N).unwrap();
    set_std_buffering(OutBuf::L).unwrap();
    set_std_buffering(OutBuf::B).unwrap();
}

#[test]
fn invalid_setvbuf_returns_error() {
    unsafe {
        let file = libc::tmpfile();
        assert!(!file.is_null());
        assert!(set_stream_buffer(file, -1).is_err());
        libc::fclose(file);
    }
}

#[test]
fn null_stream_returns_error() {
    assert!(set_stream_buffer(ptr::null_mut(), libc::_IONBF).is_err());
}

#[test]
fn stderr_failure_restores_stdout() {
    unsafe {
        let out = stdout_ptr();
        let size = mem::size_of::<libc::FILE>();
        let mut before = vec![0u8; size];
        ptr::copy_nonoverlapping(out as *const u8, before.as_mut_ptr(), size);
        let res = set_std_buffering_for_test(libc::_IONBF, out, ptr::null_mut());
        assert!(matches!(res, Err(StdBufferError::Stderr(_))));
        let mut after = vec![0u8; size];
        ptr::copy_nonoverlapping(out as *const u8, after.as_mut_ptr(), size);
        assert_eq!(before, after);
    }
}

#[test]
fn stdout_failure_reports_error() {
    unsafe {
        let err = stderr_ptr();
        let res = set_std_buffering_for_test(libc::_IONBF, ptr::null_mut(), err);
        assert!(matches!(res, Err(StdBufferError::Stdout(_))));
    }
}

#[test]
fn both_fail_reports_both() {
    unsafe {
        let res = set_std_buffering_for_test(libc::_IONBF, ptr::null_mut(), ptr::null_mut());
        assert!(matches!(res, Err(StdBufferError::Both { .. })));
    }
}

#[cfg(windows)]
#[test]
fn cli_outbuf_changes_buffering() {
    use assert_cmd::Command;

    for mode in ["N", "L", "B"] {
        Command::cargo_bin("oc-rsync")
            .unwrap()
            .args([&format!("--outbuf={mode}"), "--version"])
            .assert()
            .success();
    }
}
