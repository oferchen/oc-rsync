// tests/windows.rs
#![cfg(windows)]

use meta::{Metadata, Options};
use std::fs;
use std::io::ErrorKind;
use std::thread::sleep;
use std::time::Duration;
use tempfile::tempdir;
use transport::ssh::SshStdioTransport;
use walk::normalize_path;

#[test]
fn ssh_transport_unavailable() {
    let err = SshStdioTransport::spawn("ssh", []).expect_err("ssh available");
    assert_eq!(err.kind(), ErrorKind::Unsupported);
}

#[test]
fn normalize_extended_paths() {
    let tmp = std::env::temp_dir();
    let p = normalize_path(&tmp);
    assert!(p.as_os_str().to_string_lossy().starts_with(r"\\?\"));
}

#[test]
fn metadata_roundtrip_preserves_times_and_acl() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("file.txt");
    fs::write(&path, b"hi").unwrap();

    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&path, perms).unwrap();

    let opts = Options {
        perms: true,
        times: true,
        atimes: true,
        crtimes: true,
        ..Default::default()
    };
    let meta = Metadata::from_path(&path, opts.clone()).unwrap();

    sleep(Duration::from_secs(1));
    fs::write(&path, b"later").unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(&path, perms).unwrap();

    meta.apply(&path, opts).unwrap();
    let meta2 = fs::metadata(&path).unwrap();
    assert!(meta2.permissions().readonly());
    assert_eq!(
        filetime::FileTime::from_last_modification_time(&meta2),
        meta.mtime
    );
}
