// crates/engine/tests/max_delete.rs
use std::fs;

use compress::available_codecs;
use engine::{sync, DeleteMode, SyncOptions};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn caps_extraneous_deletions() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(dst.join("extra.txt"), b"data").unwrap();

    let opts = SyncOptions {
        delete: Some(DeleteMode::Before),
        max_delete: Some(0),
        ..Default::default()
    };
    let err = sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(None),
        &opts,
    )
    .unwrap_err();
    assert!(format!("{}", err).contains("max-delete"));
    assert!(dst.join("extra.txt").exists());

    let opts = SyncOptions {
        delete: Some(DeleteMode::Before),
        max_delete: Some(1),
        ..Default::default()
    };
    let stats = sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(None),
        &opts,
    )
    .unwrap();
    assert_eq!(stats.files_deleted, 1);
    assert!(!dst.join("extra.txt").exists());
}

#[test]
fn caps_missing_arg_deletions() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("missing.txt");
    let dst = tmp.path().join("dst.txt");
    fs::write(&dst, b"data").unwrap();

    let opts = SyncOptions {
        delete_missing_args: true,
        max_delete: Some(0),
        ..Default::default()
    };
    let err = sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(None),
        &opts,
    )
    .unwrap_err();
    assert!(format!("{}", err).contains("max-delete"));
    assert!(dst.exists());

    let opts = SyncOptions {
        delete_missing_args: true,
        max_delete: Some(1),
        ..Default::default()
    };
    let stats = sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(None),
        &opts,
    )
    .unwrap();
    assert_eq!(stats.files_deleted, 1);
    assert!(!dst.exists());
}
