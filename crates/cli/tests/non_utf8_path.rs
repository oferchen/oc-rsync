// crates/cli/tests/non_utf8_path.rs
use oc_rsync_cli::parse_remote_spec;
use std::ffi::OsString;

#[test]
fn parses_non_utf8_path() {
    let bytes = b"nonutf8\x80path";
    let path = unsafe { OsString::from_encoded_bytes_unchecked(bytes.to_vec()) };
    assert!(parse_remote_spec(&path).is_ok());
}
