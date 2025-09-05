// crates/logging/tests/escape_path.rs
use std::fs;
use std::path::Path;

use logging::escape_path;

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

#[cfg(unix)]
fn invalid_name() -> std::ffi::OsString {
    std::ffi::OsString::from_vec(vec![b'f', 0xff, b'f'])
}

#[cfg(unix)]
#[test]
fn escape_path_matches_rsync_default_fixture() {
    let name = invalid_name();
    let expected =
        fs::read_to_string("../../tests/fixtures/rsync-send-nonascii-default.txt").unwrap();
    let expected = expected.trim_end().trim_start_matches("send");
    let out = escape_path(Path::new(&name), false);
    assert_eq!(out, expected);
}

#[cfg(unix)]
#[test]
fn escape_path_matches_rsync_8bit_fixture() {
    let name = invalid_name();
    let expected = fs::read_to_string("../../tests/fixtures/rsync-send-nonascii-8bit.txt").unwrap();
    let expected = expected.trim_end().trim_start_matches("send");
    let out = escape_path(Path::new(&name), true);
    assert_eq!(out, expected);
}
