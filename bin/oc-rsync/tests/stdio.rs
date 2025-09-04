// bin/oc-rsync/tests/stdio.rs
#[path = "../src/stdio.rs"]
mod stdio;

use oc_rsync_cli::options::OutBuf;
use std::ptr;
use stdio::{set_stdout_buffering, set_stream_buffer, stdout_stream};

#[test]
fn mode_changes_ok() {
    set_stdout_buffering(OutBuf::N).unwrap();
    set_stdout_buffering(OutBuf::L).unwrap();
    set_stdout_buffering(OutBuf::B).unwrap();
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

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn unix_stdout_stream_is_valid() {
    let stream = stdout_stream().unwrap();
    assert!(!stream.as_ptr().is_null());
}

#[cfg(target_os = "macos")]
#[test]
fn macos_stdout_stream_is_valid() {
    let stream = stdout_stream().unwrap();
    assert!(!stream.as_ptr().is_null());
}

#[cfg(windows)]
#[test]
fn windows_stdout_stream_is_valid() {
    let stream = stdout_stream().unwrap();
    assert!(!stream.as_ptr().is_null());
}
