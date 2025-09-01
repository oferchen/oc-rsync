// crates/engine/tests/links.rs
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::MetadataExt;

use compress::available_codecs;
use engine::{sync, SyncOptions};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn hard_links_grouped() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file1 = src.join("a");
    fs::write(&file1, b"hi").unwrap();
    let file2 = src.join("b");
    fs::hard_link(&file1, &file2).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            hard_links: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta1 = fs::symlink_metadata(dst.join("a")).unwrap();
    let meta2 = fs::symlink_metadata(dst.join("b")).unwrap();
    assert_eq!(meta1.ino(), meta2.ino());
}

#[test]
fn copy_links_requires_referent() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let sl = src.join("link");
    std::os::unix::fs::symlink("missing", &sl).unwrap();
    let res = sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            copy_links: true,
            ..Default::default()
        },
    );
    assert!(res.is_err());
}
