// crates/engine/tests/specials.rs
#![doc = "Device tests require `CAP_MKNOD`."]
#![cfg(unix)]
#![allow(
    clippy::needless_return,
    clippy::single_match,
    clippy::collapsible_if,
    clippy::redundant_pattern_matching,
    clippy::needless_borrows_for_generic_args
)]

use std::convert::TryInto;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};

use compress::available_codecs;
use engine::{SyncOptions, sync};
use filters::Matcher;
use nix::sys::stat::{Mode, SFlag, mknod};
use nix::unistd::mkfifo;
use tempfile::tempdir;

mod tests;
#[test]
fn devices_roundtrip() {
    if !tests::requires_capability(tests::CapabilityCheck::CapMknod) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let dev = src.join("null");
    #[allow(clippy::useless_conversion)]
    mknod(
        &dev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        meta::makedev(1, 3).try_into().unwrap(),
    )
    .unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            devices: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::symlink_metadata(dst.join("null")).unwrap();
    assert!(meta.file_type().is_char_device());
    assert_eq!(meta.rdev(), meta::makedev(1, 3));
}

#[test]
fn copy_devices_creates_regular_files() {
    if !tests::requires_capability(tests::CapabilityCheck::CapMknod) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let dev = src.join("null");
    #[allow(clippy::useless_conversion)]
    mknod(
        &dev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        meta::makedev(1, 3).try_into().unwrap(),
    )
    .unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            copy_devices: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("null")).unwrap();
    assert!(meta.is_file());
    assert_eq!(meta.len(), 0);
    assert!(!meta.file_type().is_char_device());
}

#[test]
fn copy_devices_handles_zero() {
    if !tests::requires_capability(tests::CapabilityCheck::CapMknod) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let dev = src.join("zero");
    #[allow(clippy::useless_conversion)]
    mknod(
        &dev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        meta::makedev(1, 5).try_into().unwrap(),
    )
    .unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            copy_devices: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("zero")).unwrap();
    assert!(meta.is_file());
    assert_eq!(meta.len(), 0);
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
        &available_codecs(),
        &SyncOptions {
            specials: true,
            ..Default::default()
        },
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
        &available_codecs(),
        &SyncOptions {
            sparse: true,
            ..Default::default()
        },
    )
    .unwrap();
    let src_meta = fs::metadata(&sp).unwrap();
    let dst_meta = fs::metadata(dst.join("sparse")).unwrap();
    assert_eq!(src_meta.len(), dst_meta.len());
    assert_eq!(src_meta.blocks(), dst_meta.blocks());
    assert!(src_meta.blocks() * 512 < src_meta.len());
    assert!(dst_meta.blocks() * 512 < dst_meta.len());
}

#[test]
fn sparse_creation_from_zeros() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let zs = src.join("zeros");
    {
        let mut f = File::create(&zs).unwrap();
        f.write_all(&vec![0u8; 1 << 20]).unwrap();
    }
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            sparse: true,
            ..Default::default()
        },
    )
    .unwrap();
    let src_meta = fs::metadata(&zs).unwrap();
    let dst_meta = fs::metadata(dst.join("zeros")).unwrap();
    assert_eq!(src_meta.len(), dst_meta.len());
    assert!(dst_meta.blocks() < src_meta.blocks());
    assert!(dst_meta.blocks() * 512 < dst_meta.len());
}

#[test]
fn sparse_middle_hole() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let sp = src.join("sparse");
    {
        let mut f = File::create(&sp).unwrap();
        f.write_all(b"start").unwrap();
        f.seek(SeekFrom::Start((1 << 20) + 5)).unwrap();
        f.write_all(b"end").unwrap();
    }
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            sparse: true,
            ..Default::default()
        },
    )
    .unwrap();
    let src_meta = fs::metadata(&sp).unwrap();
    if src_meta.blocks() * 512 >= src_meta.len() {
        eprintln!("skipping test: filesystem lacks sparse-file support");
        return;
    }
    let dst_meta = fs::metadata(dst.join("sparse")).unwrap();
    assert_eq!(src_meta.len(), dst_meta.len());
    assert_eq!(src_meta.blocks(), dst_meta.blocks());
    assert!(dst_meta.blocks() * 512 < dst_meta.len());
}

#[test]
fn sparse_trailing_hole() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let sp = src.join("sparse");
    {
        let mut f = File::create(&sp).unwrap();
        f.write_all(b"start").unwrap();
        f.set_len(1 << 20).unwrap();
    }
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            sparse: true,
            ..Default::default()
        },
    )
    .unwrap();
    let src_meta = fs::metadata(&sp).unwrap();
    if src_meta.blocks() * 512 >= src_meta.len() {
        eprintln!("skipping test: filesystem lacks sparse-file support");
        return;
    }
    let dst_meta = fs::metadata(dst.join("sparse")).unwrap();
    assert_eq!(src_meta.len(), dst_meta.len());
    assert_eq!(src_meta.blocks(), dst_meta.blocks());
    assert!(dst_meta.blocks() * 512 < dst_meta.len());
}
