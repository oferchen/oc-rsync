// tests/filter_basic.rs
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
fn include_from_from0_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("a.txt"), "hi").unwrap();
    fs::write(src.join("b.log"), "nope").unwrap();
    fs::write(src.join("c.txt"), "hi").unwrap();

    let list = tmp.path().join("list");
    fs::write(&list, b"a.txt\0c.txt\0").unwrap();

    let src_arg = format!("{}/", src.display());
    let status = std::process::Command::new(cargo_bin("oc-rsync"))
        .args([
            "-r",
            "--from0",
            "--include-from",
            list.to_str().unwrap(),
            "--exclude",
            "*",
            &src_arg,
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--include-from",
            list.to_str().unwrap(),
            "--exclude",
            "*",
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = std::process::Command::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn exclude_from_from0_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("keep.txt"), "hi").unwrap();
    fs::write(src.join("drop.txt"), "nope").unwrap();

    let list = tmp.path().join("list");
    fs::write(&list, b"drop.txt\0").unwrap();

    let src_arg = format!("{}/", src.display());
    let status = std::process::Command::new(cargo_bin("oc-rsync"))
        .args([
            "-r",
            "--from0",
            "--exclude-from",
            list.to_str().unwrap(),
            &src_arg,
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--exclude-from",
            list.to_str().unwrap(),
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = std::process::Command::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn include_exclude_from_order() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let inc_first = tmp.path().join("inc_first");
    let exc_first = tmp.path().join("exc_first");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&inc_first).unwrap();
    fs::create_dir_all(&exc_first).unwrap();

    fs::write(src.join("keep.txt"), "hi").unwrap();
    fs::write(src.join("omit.txt"), "no").unwrap();

    let inc = tmp.path().join("inc.lst");
    fs::write(&inc, "keep.txt\n").unwrap();
    let exc = tmp.path().join("exc.lst");
    fs::write(&exc, "*\n").unwrap();

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
            inc_first.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(inc_first.join("keep.txt").exists());
    assert!(!inc_first.join("omit.txt").exists());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--exclude-from",
            exc.to_str().unwrap(),
            "--include-from",
            inc.to_str().unwrap(),
            &src_arg,
            exc_first.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!exc_first.join("keep.txt").exists());
    assert!(!exc_first.join("omit.txt").exists());
}

#[test]
fn include_from_before_exclude_allows_file() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("keep.txt"), b"k").unwrap();
    fs::write(src.join("skip.txt"), b"s").unwrap();
    let inc = dir.path().join("inc.lst");
    fs::write(&inc, "keep.txt\n").unwrap();

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

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("skip.txt").exists());
}

#[test]
fn exclude_before_include_from_skips_file() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("keep.txt"), b"k").unwrap();
    fs::write(src.join("skip.txt"), b"s").unwrap();
    let inc = dir.path().join("inc.lst");
    fs::write(&inc, "keep.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--exclude",
            "*",
            "--include-from",
            inc.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("keep.txt").exists());
    assert!(!dst.join("skip.txt").exists());
}

#[test]
fn include_from_mixed_with_include_and_exclude() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("keep1.txt"), b"k1").unwrap();
    fs::write(src.join("keep2.txt"), b"k2").unwrap();
    fs::write(src.join("skip.txt"), b"s").unwrap();
    let inc = dir.path().join("inc.lst");
    fs::write(&inc, "keep1.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include-from",
            inc.to_str().unwrap(),
            "--exclude",
            "*",
            "--include",
            "keep2.txt",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("keep1.txt").exists());
    assert!(!dst.join("keep2.txt").exists());
    assert!(!dst.join("skip.txt").exists());
}

#[test]
fn cvs_exclude_skips_ignored_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::create_dir_all(src.join(".git")).unwrap();
    fs::write(src.join(".git/file"), "git").unwrap();
    fs::write(src.join("keep.txt"), "keep").unwrap();
    fs::write(src.join("core"), "core").unwrap();
    fs::write(src.join("foo.o"), "obj").unwrap();
    fs::write(src.join("env_ignored"), "env").unwrap();
    fs::write(src.join("home_ignored"), "home").unwrap();
    fs::write(src.join("local_ignored"), "local").unwrap();
    fs::write(src.join(".cvsignore"), "local_ignored\n").unwrap();

    let sub = src.join("nested");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("keep.txt"), "keep").unwrap();
    fs::write(sub.join("core"), "core").unwrap();

    let home = tempdir().unwrap();
    fs::write(home.path().join(".cvsignore"), "home_ignored\n").unwrap();

    let src_arg = format!("{}/", src.display());
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.env("CVSIGNORE", "env_ignored");
    cmd.env("HOME", home.path());
    cmd.args([
        "--recursive",
        "--cvs-exclude",
        &src_arg,
        dst.to_str().unwrap(),
    ]);
    cmd.assert().success();

    assert!(dst.join("keep.txt").exists());
    assert!(dst.join("nested/keep.txt").exists());
    assert!(!dst.join("core").exists());
    assert!(!dst.join("nested/core").exists());
    assert!(!dst.join("foo.o").exists());
    assert!(!dst.join("env_ignored").exists());
    assert!(!dst.join("home_ignored").exists());
    assert!(!dst.join("local_ignored").exists());
    assert!(dst.join(".cvsignore").exists());
}
