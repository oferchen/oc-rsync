// crates/engine/tests/cleanup.rs
use std::fs;

use compress::available_codecs;
use engine::{sync, SyncOptions};
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
        &available_codecs(None),
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
        &available_codecs(None),
        &SyncOptions {
            temp_dir: Some(tmpdir.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(dst.join("file").exists());
    assert!(!tmpdir.exists());
}
