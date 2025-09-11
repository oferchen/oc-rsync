// tests/filter_merge.rs
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
fn filter_merge_from0_matches_filter_file() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let merge_dst = tmp.path().join("merge");
    let file_dst = tmp.path().join("file");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&merge_dst).unwrap();
    fs::create_dir_all(&file_dst).unwrap();

    fs::write(src.join("a.txt"), "hi").unwrap();
    fs::write(src.join("b.log"), "no").unwrap();
    fs::write(src.join("c.txt"), "hi").unwrap();

    let filter = tmp.path().join("filters");
    fs::write(&filter, b"+ *.txt\0- *\0").unwrap();

    let src_arg = format!("{}/", src.display());
    let status = std::process::Command::new(cargo_bin("oc-rsync"))
        .args([
            "-r",
            "--from0",
            "--filter",
            &format!("merge {}", filter.display()),
            &src_arg,
            merge_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--filter-file",
            filter.to_str().unwrap(),
            &src_arg,
            file_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = std::process::Command::new("diff")
        .arg("-r")
        .arg(&merge_dst)
        .arg(&file_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn filter_file_from0_stdin() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(src.join("keep.txt"), "hi").unwrap();
    fs::write(src.join("omit.txt"), "no").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--filter-file",
            "-",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .write_stdin(b"+ /keep.txt\0- *\0" as &[u8])
        .assert()
        .success();

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("omit.txt").exists());
}

#[test]
fn per_dir_merge_filters() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(src.join("keep.txt"), "hi").unwrap();
    fs::write(src.join("omit.log"), "no").unwrap();
    fs::write(src.join("sub").join("keep2.txt"), "hi").unwrap();
    fs::write(src.join("sub").join("omit2.txt"), "no").unwrap();

    fs::write(src.join(".rsync-filter"), b"- *.log\n").unwrap();
    fs::write(src.join("sub").join(".rsync-filter"), b"- omit2.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--recursive", "-F", "-F", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    assert!(dst.join("keep.txt").exists());
    assert!(dst.join("sub").join("keep2.txt").exists());
    assert!(!dst.join("omit.log").exists());
    assert!(!dst.join("sub").join("omit2.txt").exists());
}

#[test]
fn filter_ancestor_expansion() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("a/b")).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(src.join("a/b/keep.txt"), "hi").unwrap();
    fs::write(src.join("a/b/omit.txt"), "no").unwrap();
    fs::write(src.join(".rsync-filter"), b"+ /a/b/keep.txt\n- *\n").unwrap();
    fs::write(src.join("a/.rsync-filter"), b"- omit.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--recursive", "-F", "-F", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    assert!(dst.join("a").join("b").join("keep.txt").exists());
    assert!(!dst.join("a").join("b").join("omit.txt").exists());
}

#[test]
fn per_dir_merge_can_override_later_include() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join(".rsync-filter"), "- skip.txt\n").unwrap();
    std::fs::write(src.join("skip.txt"), b"s").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--filter",
            ": .rsync-filter",
            "--include",
            "skip.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("skip.txt").exists());
}

#[test]
fn include_before_per_dir_merge_allows_file() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join(".rsync-filter"), "- skip.txt\n").unwrap();
    std::fs::write(src.join("skip.txt"), b"s").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include",
            "skip.txt",
            "--filter",
            ": .rsync-filter",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("skip.txt").exists());
}

#[test]
fn rsync_filter_merges_across_directories() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("root.txt"), b"r").unwrap();
    fs::write(src.join(".rsync-filter"), "- root.txt\n").unwrap();
    fs::write(src.join("sub/keep.txt"), b"k").unwrap();
    fs::write(src.join("sub/skip.txt"), b"s").unwrap();
    fs::write(src.join("sub/.rsync-filter"), "- skip.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--filter",
            ": .rsync-filter",
            "--include",
            "sub/***",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("root.txt").exists());
    assert!(dst.join("sub/keep.txt").exists());
    assert!(!dst.join("sub/skip.txt").exists());
}

#[test]
fn filter_file_zero_separated_from_stdin() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep.txt"), b"k").unwrap();
    std::fs::write(src.join("omit.txt"), b"o").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--filter-file",
            "-",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .write_stdin(b"+ /keep.txt\0- *\0" as &[u8])
        .assert()
        .success();

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("omit.txt").exists());
}

#[test]
fn filter_file_zero_separated_from_stdin_overrides_cvs_exclude() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(src.join(".git")).unwrap();
    std::fs::write(src.join(".git/keep.txt"), b"k").unwrap();
    std::fs::write(src.join("other.txt"), b"o").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--filter-file",
            "-",
            "--cvs-exclude",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .write_stdin(b"+.git/***\0-*\0" as &[u8])
        .assert()
        .success();

    assert!(dst.join(".git/keep.txt").exists());
    assert!(!dst.join("other.txt").exists());
}
