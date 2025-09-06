// crates/engine/tests/cleanup.rs
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use compress::available_codecs;
use engine::{SyncOptions, sync};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn removes_partial_dir_after_sync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();

    let partial = tmp.path().join("partials");
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            partial: true,
            partial_dir: Some(partial.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(dst.join("file").exists());
    assert!(!partial.exists());
}

#[test]
fn removes_temp_dir_after_sync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();

    let tmpdir = tmp.path().join("tmpdir");
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            temp_dir: Some(tmpdir.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(dst.join("file").exists());
    assert!(!tmpdir.exists());
}

#[test]
#[cfg(unix)]
fn cleans_up_temp_dir_on_rename_failure() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    let tmpdir = tmp.path().join("tmpdir");
    fs::create_dir_all(&tmpdir).unwrap();
    let mut perms = fs::metadata(&dst).unwrap().permissions();
    perms.set_mode(0o500);
    fs::set_permissions(&dst, perms).unwrap();
    let res = sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            temp_dir: Some(tmpdir.clone()),
            ..Default::default()
        },
    );
    assert!(res.is_err());
    fs::set_permissions(&dst, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(fs::read_dir(&tmpdir).unwrap().next().is_none());
}
