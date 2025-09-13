// crates/engine/tests/xattrs.rs
#![doc = "Xattr tests skip when unsupported."]
#![cfg(unix)]
#![allow(
    clippy::needless_return,
    clippy::single_match,
    clippy::collapsible_if,
    clippy::redundant_pattern_matching,
    clippy::needless_borrows_for_generic_args
)]

use std::fs;

use compress::available_codecs;
use engine::{SyncOptions, sync};
use filters::Matcher;
use tempfile::tempdir;

mod tests;
#[cfg(feature = "xattr")]
#[test]
fn xattrs_roundtrip() {
    if let Err(e) = engine::xattrs::ensure_supported() {
        println!("Skipping test: {e}");
        return;
    }
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
        &available_codecs(),
        &SyncOptions {
            xattrs: true,
            ..Default::default()
        },
    )
    .unwrap();
    let val = xattr::get(dst.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
}

#[cfg(feature = "xattr")]
#[test]
fn symlink_xattrs_roundtrip() {
    if let Err(e) = engine::xattrs::ensure_supported() {
        println!("Skipping test: {e}");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    std::os::unix::fs::symlink("file", src.join("link")).unwrap();
    xattr::set(src.join("link"), "user.test", b"val").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            xattrs: true,
            links: true,
            ..Default::default()
        },
    )
    .unwrap();
    let val = xattr::get(dst.join("link"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
}
#[cfg(feature = "xattr")]
#[test]
fn fake_super_stores_xattrs() {
    if let Err(e) = engine::xattrs::ensure_supported() {
        println!("Skipping test: {e}");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            perms: true,
            fake_super: true,
            ..Default::default()
        },
    )
    .unwrap();
    let dst_file = dst.join("file");
    assert!(xattr::get(&dst_file, "user.rsync.uid").unwrap().is_some());
}

#[cfg(all(unix, feature = "xattr"))]
#[test]
fn super_overrides_fake_super() {
    if !tests::requires_capability(tests::CapabilityCheck::CapChown) {
        return;
    }
    if let Err(e) = engine::xattrs::ensure_supported() {
        println!("Skipping test: {e}");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            perms: true,
            fake_super: true,
            super_user: true,
            ..Default::default()
        },
    )
    .unwrap();
    let dst_file = dst.join("file");
    assert!(xattr::get(&dst_file, "user.rsync.uid").unwrap().is_none());
}

#[cfg(feature = "xattr")]
#[test]
fn xattrs_roundtrip_fake_super() {
    if let Err(e) = engine::xattrs::ensure_supported() {
        println!("Skipping test: {e}");
        return;
    }
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
        &available_codecs(),
        &SyncOptions {
            xattrs: true,
            ..Default::default()
        },
    )
    .unwrap();
    let val = xattr::get(dst.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
}
