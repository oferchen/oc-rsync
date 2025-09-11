// tests/filter_lists.rs
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
fn exclude_from_file_skips_patterns() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep.txt"), b"k").unwrap();
    std::fs::write(src.join("skip.log"), b"s").unwrap();
    let list = dir.path().join("exclude.txt");
    std::fs::write(&list, "*.log\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--exclude-from",
            list.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("skip.log").exists());
}

#[test]
fn include_from_list_transfers_nested_paths() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("a/b")).unwrap();
    fs::create_dir_all(src.join("a/d/sub")).unwrap();
    fs::write(src.join("a/b/file.txt"), b"f").unwrap();
    fs::write(src.join("a/b/other.txt"), b"o").unwrap();
    fs::write(src.join("a/d/sub/nested.txt"), b"n").unwrap();
    fs::write(src.join("unlisted.txt"), b"u").unwrap();
    let inc = dir.path().join("inc.lst");
    fs::write(&inc, "a/b/file.txt\na/d/\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include-from",
            inc.to_str().unwrap(),
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("a/b/file.txt").exists());
    assert!(dst.join("a/d/sub/nested.txt").exists());
    assert!(!dst.join("a/b/other.txt").exists());
    assert!(!dst.join("unlisted.txt").exists());
}

#[test]
fn include_from_list_nested_file_excludes_siblings() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("a/b")).unwrap();
    fs::create_dir_all(src.join("a/c")).unwrap();
    fs::write(src.join("a/b/file.txt"), b"f").unwrap();
    fs::write(src.join("a/b/other.txt"), b"o").unwrap();
    fs::write(src.join("a/c/unrelated.txt"), b"u").unwrap();
    let inc = dir.path().join("inc.lst");
    fs::write(&inc, "a/b/file.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include-from",
            inc.to_str().unwrap(),
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("a/b/file.txt").exists());
    assert!(!dst.join("a/b/other.txt").exists());
    assert!(!dst.join("a/c/unrelated.txt").exists());
}

#[test]
fn exclude_from_zero_separated_list_with_crlf() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep.txt"), b"k").unwrap();
    std::fs::write(src.join("skip.log"), b"s").unwrap();
    let list = dir.path().join("exclude.lst");
    std::fs::write(&list, b"*.log\r\n\0").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--exclude-from",
            list.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("skip.log").exists());
}

#[test]
fn include_from_zero_separated_list_with_crlf() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep.txt"), b"k").unwrap();
    std::fs::write(src.join("skip.txt"), b"s").unwrap();
    let inc = dir.path().join("include.lst");
    let exc = dir.path().join("exclude.lst");
    std::fs::write(&inc, b"keep.txt\r\n\0").unwrap();
    std::fs::write(&exc, b"*\r\n\0").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--include-from",
            inc.to_str().unwrap(),
            "--exclude-from",
            exc.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("skip.txt").exists());
}

#[test]
fn include_from_zero_separated_list_allows_hash() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep#me.txt"), b"k").unwrap();
    std::fs::write(src.join("skip.txt"), b"s").unwrap();
    let inc = dir.path().join("include.lst");
    std::fs::write(&inc, b"keep#me.txt\0").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--include-from",
            inc.to_str().unwrap(),
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("keep#me.txt").exists());
    assert!(!dst.join("skip.txt").exists());
}

#[test]
fn include_from_creates_needed_dirs() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("a/b/c")).unwrap();
    fs::write(src.join("a/b/c/keep.txt"), "hi").unwrap();
    fs::write(src.join("a/b/c/omit.txt"), "no").unwrap();
    let list = tmp.path().join("list");
    fs::write(&list, "a/b/c/keep.txt\n").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include-from",
            list.to_str().unwrap(),
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("a/b/c/keep.txt").is_file());
    assert!(!dst.join("a/b/c/omit.txt").exists());
}

#[test]
fn include_from_nested_path_allows_parent_dirs() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("nested/dir")).unwrap();
    fs::write(src.join("nested/dir/keep.txt"), b"k").unwrap();
    fs::write(src.join("nested/dir/skip.txt"), b"s").unwrap();
    let inc = dir.path().join("inc.lst");
    fs::write(&inc, "nested/dir/keep.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include-from",
            inc.to_str().unwrap(),
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("nested/dir/keep.txt").exists());
    assert!(!dst.join("nested/dir/skip.txt").exists());
}

#[test]
fn include_from_file_allows_patterns() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep.txt"), b"k").unwrap();
    std::fs::write(src.join("skip.txt"), b"s").unwrap();
    let inc = dir.path().join("include.txt");
    std::fs::write(&inc, "keep.txt\n").unwrap();
    let exc = dir.path().join("exclude.txt");
    std::fs::write(&exc, "*\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include-from",
            inc.to_str().unwrap(),
            "--exclude-from",
            exc.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("skip.txt").exists());
}
