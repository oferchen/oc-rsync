// crates/engine/tests/checksum.rs
use std::fs;

use compress::available_codecs;
use engine::{sync, SyncOptions};
use filetime::{set_file_mtime, FileTime};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn checksum_forces_transfer() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let src_file = src.join("file");
    let dst_file = dst.join("file");
    fs::write(&src_file, b"aaaa").unwrap();
    fs::write(&dst_file, b"bbbb").unwrap();

    let mtime = FileTime::from_unix_time(1_000_000, 0);
    set_file_mtime(&src_file, mtime).unwrap();
    set_file_mtime(&dst_file, mtime).unwrap();

    let stats = sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions::default(),
    )
    .unwrap();
    assert_eq!(stats.files_transferred, 0);
    assert_eq!(fs::read(&dst_file).unwrap(), b"bbbb");

    let stats = sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions {
            checksum: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(stats.files_transferred, 1);
    assert_eq!(fs::read(&dst_file).unwrap(), b"aaaa");
}

#[test]
fn checksum_skips_transfer_when_unchanged() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let src_file = src.join("file");
    let dst_file = dst.join("file");
    fs::write(&src_file, b"aaaa").unwrap();
    fs::write(&dst_file, b"aaaa").unwrap();

    let src_mtime = FileTime::from_unix_time(1_000_000, 0);
    let dst_mtime = FileTime::from_unix_time(2_000_000, 0);
    set_file_mtime(&src_file, src_mtime).unwrap();
    set_file_mtime(&dst_file, dst_mtime).unwrap();

    let stats = sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions {
            checksum: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(stats.files_transferred, 0);
    assert_eq!(fs::read(&dst_file).unwrap(), b"aaaa");
}
