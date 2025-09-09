// crates/meta/tests/non_unix_stub.rs
#![cfg(all(not(unix), not(target_os = "windows")))]

use std::io::ErrorKind;
use std::path::Path;

use filetime::FileTime;
use meta::{HardLinks, Metadata, Options, hard_link_id, read_acl, store_fake_super, write_acl};

#[test]
fn metadata_from_path_is_unsupported() {
    let err = Metadata::from_path(Path::new("."), Options::default()).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
}

#[test]
fn metadata_apply_is_unsupported() {
    let md = Metadata {
        uid: 0,
        gid: 0,
        mode: 0,
        mtime: FileTime::from_unix_time(0, 0),
        atime: None,
        crtime: None,
        xattrs: Vec::new(),
        acl: Vec::new(),
        default_acl: Vec::new(),
    };
    let err = md.apply(Path::new("."), Options::default()).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
}

#[test]
fn hard_link_operations_are_unsupported() {
    let mut hl = HardLinks::default();
    assert!(!hl.register(1, Path::new("foo")));
    let err = hard_link_id(0, 0).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
    let err = store_fake_super(Path::new("foo"), 0, 0, 0).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
}

#[test]
fn acl_operations_are_unsupported() {
    let err = read_acl(Path::new("foo"), false).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
    let err = write_acl(Path::new("foo"), &[], &[], false, false).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
}
