// tests/bin_stdio.rs
#[path = "../src/bin/oc-rsync/stdio.rs"]
mod stdio;

use oc_rsync_cli::options::OutBuf;
use std::ptr;
use stdio::{set_std_buffering, set_stream_buffer};

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
