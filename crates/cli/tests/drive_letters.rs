// crates/cli/tests/drive_letters.rs
#![cfg(windows)]

use oc_rsync_cli::{parse_remote_spec, RemoteSpec};

#[test]
fn drive_letter_without_separator_is_local() {
    let spec = parse_remote_spec("C:").unwrap();
    assert!(matches!(spec, RemoteSpec::Local(_)));
}

#[test]
fn drive_letter_with_separator_is_local() {
    for path in ["C:/", r"C:\"] {
        let spec = parse_remote_spec(path).unwrap();
        assert!(matches!(spec, RemoteSpec::Local(_)));
    }
}
