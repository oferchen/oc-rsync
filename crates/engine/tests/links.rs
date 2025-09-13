// crates/engine/tests/links.rs
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::MetadataExt;

use compress::available_codecs;
use engine::{SyncOptions, sync};
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
    let file3 = src.join("c");
    fs::hard_link(&file1, &file3).unwrap();
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
    let meta3 = fs::symlink_metadata(dst.join("c")).unwrap();
    assert_eq!(meta1.ino(), meta2.ino());
    assert_eq!(meta1.ino(), meta3.ino());
}

#[test]
fn hard_links_existing_dest() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file1 = src.join("a");
    fs::write(&file1, b"hi").unwrap();
    let file2 = src.join("b");
    fs::hard_link(&file1, &file2).unwrap();

    let dst_a = dst.join("a");
    fs::write(&dst_a, b"junk").unwrap();

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

    let ino1 = fs::metadata(dst.join("a")).unwrap().ino();
    let ino2 = fs::metadata(dst.join("b")).unwrap().ino();
    assert_eq!(ino1, ino2);
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
