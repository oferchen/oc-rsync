#![cfg(unix)]

use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};

use compress::available_codecs;
use engine::{sync, SyncOptions};
use filetime::{set_file_mtime, FileTime};
use filters::Matcher;
use nix::sys::stat::{mknod, makedev, Mode, SFlag};
use nix::unistd::{chown, mkfifo, Gid, Uid};
use tempfile::tempdir;
use xattr;

#[test]
fn perms_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions { perms: true, ..Default::default() },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o640);
}

#[test]
fn times_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let mtime = FileTime::from_unix_time(1_000_000, 0);
    set_file_mtime(&file, mtime).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions { times: true, ..Default::default() },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("file")).unwrap();
    let dst_mtime = FileTime::from_last_modification_time(&meta);
    assert_eq!(dst_mtime, mtime);
}

#[test]
fn owner_group_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    chown(&file, Some(Uid::from_raw(1000)), Some(Gid::from_raw(1000))).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions { owner: true, group: true, ..Default::default() },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.uid(), 1000);
    assert_eq!(meta.gid(), 1000);
}

#[test]
fn links_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("target"), b"t").unwrap();
    std::os::unix::fs::symlink("target", src.join("link")).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions { links: true, ..Default::default() },
    )
    .unwrap();
    let meta = fs::symlink_metadata(dst.join("link")).unwrap();
    assert!(meta.file_type().is_symlink());
    assert_eq!(
        fs::read_link(dst.join("link")).unwrap(),
        std::path::PathBuf::from("target")
    );
}

#[test]
fn hard_links_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let f1 = src.join("f1");
    fs::write(&f1, b"hi").unwrap();
    let f2 = src.join("f2");
    fs::hard_link(&f1, &f2).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions { hard_links: true, ..Default::default() },
    )
    .unwrap();
    let m1 = fs::metadata(dst.join("f1")).unwrap();
    let m2 = fs::metadata(dst.join("f2")).unwrap();
    assert_eq!(m1.ino(), m2.ino());
}

#[test]
fn xattrs_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    xattr::set(&file, "user.test", b"val").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions { xattrs: true, ..Default::default() },
    )
    .unwrap();
    let val = xattr::get(dst.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
}

#[test]
fn acls_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    xattr::set(&file, "user.acltest", b"acl").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions { acls: true, ..Default::default() },
    )
    .unwrap();
    let val = xattr::get(dst.join("file"), "user.acltest").unwrap().unwrap();
    assert_eq!(&val[..], b"acl");
}

#[test]
fn devices_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let dev = src.join("null");
    mknod(
        &dev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        makedev(1, 3),
    )
    .unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions { devices: true, ..Default::default() },
    )
    .unwrap();
    let meta = fs::symlink_metadata(dst.join("null")).unwrap();
    assert!(meta.file_type().is_char_device());
    assert_eq!(meta.rdev(), makedev(1, 3));
}

#[test]
fn specials_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let fifo = src.join("fifo");
    mkfifo(&fifo, Mode::from_bits_truncate(0o600)).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions { specials: true, ..Default::default() },
    )
    .unwrap();
    let meta = fs::symlink_metadata(dst.join("fifo")).unwrap();
    assert!(meta.file_type().is_fifo());
}

#[test]
fn sparse_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let sp = src.join("sparse");
    {
        let mut f = File::create(&sp).unwrap();
        f.seek(SeekFrom::Start(1 << 20)).unwrap();
        f.write_all(b"end").unwrap();
    }
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions { sparse: true, ..Default::default() },
    )
    .unwrap();
    let src_meta = fs::metadata(&sp).unwrap();
    let dst_meta = fs::metadata(dst.join("sparse")).unwrap();
    assert_eq!(src_meta.len(), dst_meta.len());
    assert_eq!(src_meta.blocks(), dst_meta.blocks());
}
