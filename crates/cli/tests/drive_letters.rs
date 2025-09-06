// crates/cli/tests/drive_letters.rs
#![cfg(windows)]

use oc_rsync_cli::{RemoteSpec, parse_remote_spec};
use std::ffi::OsStr;

#[test]
fn drive_letter_without_separator_is_local() {
    let spec = parse_remote_spec(OsStr::new("C:")).unwrap();
    assert!(matches!(spec, RemoteSpec::Local(_)));
}

#[test]
fn drive_letter_with_separator_is_local() {
    for path in ["C:/", r"C:\"] {
        let spec = parse_remote_spec(OsStr::new(path)).unwrap();
        assert!(matches!(spec, RemoteSpec::Local(_)));
    }
}
