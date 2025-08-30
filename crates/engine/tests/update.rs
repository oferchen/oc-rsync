// crates/engine/tests/update.rs
use compress::available_codecs;
use engine::{sync, SyncOptions};
use filetime::{set_file_mtime, FileTime};
use filters::Matcher;
use std::fs;
use tempfile::tempdir;

#[test]
fn update_skips_newer_dest() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.txt"), b"new").unwrap();
    fs::write(dst.join("file.txt"), b"old").unwrap();
    let src_time = FileTime::from_unix_time(1_000_000_000, 0);
    let dst_time = FileTime::from_unix_time(2_000_000_000, 0);
    set_file_mtime(src.join("file.txt"), src_time).unwrap();
    set_file_mtime(dst.join("file.txt"), dst_time).unwrap();
    let mut opts = SyncOptions::default();
    opts.update = true;
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(None),
        &opts,
    )
    .unwrap();
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"old");
}

#[test]
fn update_replaces_older_dest() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.txt"), b"new").unwrap();
    fs::write(dst.join("file.txt"), b"old").unwrap();
    let src_time = FileTime::from_unix_time(2_000_000_000, 0);
    let dst_time = FileTime::from_unix_time(1_000_000_000, 0);
    set_file_mtime(src.join("file.txt"), src_time).unwrap();
    set_file_mtime(dst.join("file.txt"), dst_time).unwrap();
    let mut opts = SyncOptions::default();
    opts.update = true;
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(None),
        &opts,
    )
    .unwrap();
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"new");
}
