use std::fs;

use compress::Codec;
use engine::{sync, SyncOptions};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn zlib_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file.txt"), b"hello world").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &[Codec::Zlib],
        &SyncOptions { compress: true, ..Default::default() },
    )
    .unwrap();
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
}

#[test]
fn zstd_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file.txt"), b"hello world").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &[Codec::Zstd],
        &SyncOptions { compress: true, ..Default::default() },
    )
    .unwrap();
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
}

#[test]
fn lz4_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file.txt"), b"hello world").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &[Codec::Lz4],
        &SyncOptions { compress: true, ..Default::default() },
    )
    .unwrap();
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
}
