// tests/filter_glob.rs
#![allow(unused_imports)]

use assert_cmd::prelude::*;
use assert_cmd::{Command, cargo::cargo_bin};
use engine::SyncOptions;
use filetime::{FileTime, set_file_mtime};
use logging::progress_formatter;
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

#[test]
fn include_glob_prunes_unmatched_dirs() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("a")).unwrap();
    fs::create_dir_all(src.join("b/sub")).unwrap();
    fs::create_dir_all(src.join("c")).unwrap();
    fs::write(src.join("a/keep1.txt"), b"hi").unwrap();
    fs::write(src.join("b/sub/keep2.txt"), b"hi").unwrap();
    fs::write(src.join("c/omit.txt"), b"no").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include",
            "**/keep?.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("a/keep1.txt").exists());
    assert!(dst.join("b/sub/keep2.txt").exists());
    assert!(!dst.join("c").exists());
}

#[test]
fn exclude_pattern_skips_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep.txt"), b"k").unwrap();
    std::fs::write(src.join("skip.log"), b"s").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--exclude",
            "*.log",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("skip.log").exists());
}

#[test]
fn include_pattern_allows_file() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep.txt"), b"k").unwrap();
    std::fs::write(src.join("skip.txt"), b"s").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include",
            "keep.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("skip.txt").exists());
    assert_eq!(fs::read_dir(&dst).unwrap().count(), 1);
}

#[test]
fn include_nested_path_allows_file() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("sub/keep.txt"), b"k").unwrap();
    fs::write(src.join("sub/skip.txt"), b"s").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include",
            "sub/keep.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("sub/keep.txt").exists());
    assert!(!dst.join("sub/skip.txt").exists());
}

#[test]
fn include_complex_glob_pattern_allows_file() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("dir1/sub")).unwrap();
    fs::create_dir_all(src.join("dir2")).unwrap();
    fs::write(src.join("dir1/sub/keep1.txt"), b"k").unwrap();
    fs::write(src.join("dir2/keep2.txt"), b"k").unwrap();
    fs::write(src.join("dir1/sub/skip.log"), b"s").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include",
            "dir*/**/keep[0-9].txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("dir1/sub/keep1.txt").exists());
    assert!(dst.join("dir2/keep2.txt").exists());
    assert!(!dst.join("dir1/sub/skip.log").exists());
}

#[test]
fn include_after_exclude_does_not_override() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep.log"), b"k").unwrap();
    std::fs::write(src.join("skip.log"), b"s").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--exclude",
            "*.log",
            "--include",
            "keep.log",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("keep.log").exists());
    assert!(!dst.join("skip.log").exists());
}

#[test]
fn include_nested_path_allows_parent_dirs() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("a/b")).unwrap();
    fs::write(src.join("a/b/keep.txt"), b"k").unwrap();
    fs::write(src.join("a/b/skip.txt"), b"s").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include",
            "a/b/keep.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("a/b/keep.txt").exists());
    assert!(!dst.join("a/b/skip.txt").exists());
}

#[test]
fn exclude_dot_anchor_only_skips_root() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(src.join("sub")).unwrap();
    std::fs::write(src.join("root.txt"), b"r").unwrap();
    std::fs::write(src.join("sub/root.txt"), b"s").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--exclude=./root.txt",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("root.txt").exists());
    assert!(dst.join("sub/root.txt").exists());
}

#[test]
fn exclude_complex_pattern_skips_nested_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(src.join("dir/sub")).unwrap();
    std::fs::write(src.join("dir/log1.txt"), b"1").unwrap();
    std::fs::write(src.join("dir/sub/log2.txt"), b"2").unwrap();
    std::fs::write(src.join("dir/sub/logx.txt"), b"x").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--exclude",
            "dir/**/log[0-9].txt",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("dir/log1.txt").exists());
    assert!(!dst.join("dir/sub/log2.txt").exists());
    assert!(dst.join("dir/sub/logx.txt").exists());
}

#[test]
fn include_complex_pattern_allows_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(src.join("dir")).unwrap();
    std::fs::write(src.join("dir/keep1.txt"), b"1").unwrap();
    std::fs::write(src.join("dir/keep2.log"), b"2").unwrap();
    std::fs::write(src.join("dir/skip.txt"), b"s").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include",
            "**/keep?.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("dir/keep1.txt").exists());
    assert!(!dst.join("dir/keep2.log").exists());
    assert!(!dst.join("dir/skip.txt").exists());
}

#[test]
fn include_from_complex_glob_matches() {
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
    let inc = tmp.path().join("include.lst");
    fs::write(&inc, "dir*/**/keep[0-9].txt\n").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include-from",
            inc.to_str().unwrap(),
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
fn nested_include_creates_needed_dirs() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("a/b/c")).unwrap();
    fs::write(src.join("a/b/c/keep.txt"), "hi").unwrap();
    fs::write(src.join("a/b/c/drop.txt"), "no").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include",
            "a/b/c/keep.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("a/b/c/keep.txt").is_file());
    assert!(!dst.join("a/b/c/drop.txt").exists());
}
