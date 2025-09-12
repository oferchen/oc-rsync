// tests/misc_patterns.rs
#![allow(unused_imports)]

use assert_cmd::prelude::*;
use assert_cmd::{Command, cargo::cargo_bin};
use engine::SyncOptions;
use filetime::{FileTime, set_file_mtime};
#[cfg(unix)]
use nix::unistd::{Gid, Uid, chown, mkfifo};
use oc_rsync_cli::{parse_iconv, spawn_daemon_session};
use predicates::prelude::PredicateBooleanExt;
use protocol::SUPPORTED_PROTOCOLS;
use serial_test::serial;
use std::fs;
use std::io::{Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tempfile::{TempDir, tempdir, tempdir_in};
#[cfg(unix)]
use users::{get_current_gid, get_current_uid, get_group_by_gid, get_user_by_uid};
mod common;
use common::read_golden;

#[allow(clippy::vec_init_then_push)]
#[allow(clippy::vec_init_then_push)]
#[test]
fn sparse_files_created() {
    use std::fs::File;
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    let zs = src.join("zeros");
    let mut f = File::create(&zs).unwrap();
    f.write_all(&vec![0u8; 1 << 18]).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--sparse", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let src_meta = std::fs::metadata(&zs).unwrap();
    if src_meta.blocks() * 512 >= src_meta.len() {
        eprintln!("skipping test: filesystem lacks sparse-file support");
        return;
    }
    let dst_meta = std::fs::metadata(dst.join("zeros")).unwrap();
    assert_eq!(src_meta.len(), dst_meta.len());
    if dst_meta.blocks() * 512 < dst_meta.len() {
        assert!(dst_meta.blocks() < src_meta.blocks());
    }
}

#[cfg(unix)]
#[test]
fn specials_preserve_fifo() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    let fifo = src.join("pipe");
    mkfifo(&fifo, nix::sys::stat::Mode::from_bits_truncate(0o600)).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--specials", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = std::fs::symlink_metadata(dst.join("pipe")).unwrap();
    assert!(meta.file_type().is_fifo());
}

#[test]
fn delete_delay_removes_extraneous_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::write(dst.join("old.txt"), b"old").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--delete-delay",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("old.txt").exists());
}

#[cfg(all(unix, feature = "xattr"))]
#[cfg(all(unix, feature = "xattr"))]
#[test]
fn super_overrides_fake_super() {
    if !Uid::effective().is_root() {
        eprintln!("skipping super_overrides_fake_super: requires root");
        return;
    }
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    let dst_dir = tmp.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    fs::write(src_dir.join("file"), b"hi").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-a",
            "--fake-super",
            "--super",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    let dst_file = dst_dir.join("file");
    assert!(xattr::get(&dst_file, "user.rsync.uid").unwrap().is_none());
}

#[test]
fn complex_glob_matches() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("dirA")).unwrap();
    fs::write(src.join("dirA/keep1.txt"), "1").unwrap();
    fs::write(src.join("dirA/skip.log"), "x").unwrap();
    fs::create_dir_all(src.join("dirB/sub")).unwrap();
    fs::write(src.join("dirB/sub/keep2.txt"), "2").unwrap();
    fs::write(src.join("dirB/sub/other.txt"), "x").unwrap();
    fs::create_dir_all(src.join("dirC/deep/deeper")).unwrap();
    fs::write(src.join("dirC/deep/deeper/keep3.txt"), "3").unwrap();
    fs::create_dir_all(src.join("otherdir")).unwrap();
    fs::write(src.join("otherdir/keep4.txt"), "4").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include",
            "dir*/**/keep[0-9].txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("dirA/keep1.txt").is_file());
    assert!(dst.join("dirB/sub/keep2.txt").is_file());
    assert!(dst.join("dirC/deep/deeper/keep3.txt").is_file());
    assert!(dst.join("dirA").is_dir());
    assert!(dst.join("dirB/sub").is_dir());
    assert!(dst.join("dirC/deep/deeper").is_dir());
    assert!(!dst.join("dirA/skip.log").exists());
    assert!(!dst.join("dirB/sub/other.txt").exists());
    assert!(!dst.join("otherdir/keep4.txt").exists());
    assert!(!dst.join("otherdir").exists());
}

#[test]
fn double_star_glob_matches() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("a/inner")).unwrap();
    fs::create_dir_all(src.join("b/deeper/more")).unwrap();
    fs::create_dir_all(src.join("c/other")).unwrap();
    fs::write(src.join("a/keep1.txt"), "1").unwrap();
    fs::write(src.join("a/inner/keep2.txt"), "2").unwrap();
    fs::write(src.join("b/deeper/more/keep3.txt"), "3").unwrap();
    fs::write(src.join("a/omit.log"), "x").unwrap();
    fs::write(src.join("a/inner/omit.log"), "x").unwrap();
    fs::write(src.join("b/deeper/more/omit.log"), "x").unwrap();
    fs::write(src.join("c/other/omit.log"), "x").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include",
            "**/keep?.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("a/keep1.txt").is_file());
    assert!(dst.join("a/inner/keep2.txt").is_file());
    assert!(dst.join("b/deeper/more/keep3.txt").is_file());
    assert!(dst.join("a").is_dir());
    assert!(dst.join("a/inner").is_dir());
    assert!(dst.join("b/deeper/more").is_dir());
    assert!(!dst.join("a/omit.log").exists());
    assert!(!dst.join("a/inner/omit.log").exists());
    assert!(!dst.join("b/deeper/more/omit.log").exists());
    assert!(!dst.join("c/other/omit.log").exists());
    assert!(!dst.join("c").exists());
}

#[test]
fn single_star_does_not_cross_directories() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("a/b")).unwrap();
    fs::write(src.join("a/file.txt"), b"1").unwrap();
    fs::write(src.join("a/b/file.txt"), b"2").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include",
            "*/file.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("a/file.txt").exists());
    assert!(dst.join("a").is_dir());
    assert!(!dst.join("a/b/file.txt").exists());
    assert!(!dst.join("a/b").exists());
}

#[test]
fn segment_star_does_not_cross_directories() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("data/sub")).unwrap();
    fs::write(src.join("data/file.txt"), b"1").unwrap();
    fs::write(src.join("data/sub/file.txt"), b"2").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include",
            "data*/file.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("data/file.txt").exists());
    assert!(dst.join("data").is_dir());
    assert!(!dst.join("data/sub/file.txt").exists());
    assert!(!dst.join("data/sub").exists());
}

#[test]
fn char_class_respects_directory_boundaries() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("1/2")).unwrap();
    fs::write(src.join("1/keep.txt"), b"k").unwrap();
    fs::write(src.join("1/2/keep.txt"), b"x").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include",
            "[0-9]/*",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("1/keep.txt").exists());
    assert!(dst.join("1").is_dir());
    assert!(dst.join("1/2").is_dir());
    assert!(!dst.join("1/2/keep.txt").exists());
}
