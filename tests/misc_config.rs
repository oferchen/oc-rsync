// tests/misc_config.rs
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
fn resumes_from_partial_file() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), b"hello").unwrap();
    std::fs::write(dst_dir.join("a.txt.partial"), b"he").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--partial", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"hello");
    assert!(!dst_dir.join("a.txt.partial").exists());
}

#[test]
fn resumes_from_partial_file_with_temp_dir() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    let tmp_dir = dir.path().join("tmp");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    std::fs::create_dir_all(&tmp_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), b"hello").unwrap();
    std::fs::write(dst_dir.join("a.txt.partial"), b"he").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--partial",
        "--temp-dir",
        tmp_dir.to_str().unwrap(),
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"hello");
    assert!(!dst_dir.join("a.txt.partial").exists());
}

#[test]
fn config_flag_prints_message() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    let cfg = dir.path().join("config");
    std::fs::write(&cfg, b"cfg").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--config",
        cfg.to_str().unwrap(),
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("using config file"));
}

#[test]
fn no_default_config_used_without_flag() {
    let dir = tempdir().unwrap();
    let home = dir.path();
    let src_dir = home.join("src");
    let dst_dir = home.join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    let cfg = home.join(".config/oc-rsync/config.toml");
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    std::fs::write(&cfg, b"cfg").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.env("HOME", home);
    cmd.args([&src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("using config file").not());
}

#[test]
fn client_rejects_port_without_daemon() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--port", "1234"])
        .assert()
        .failure();
}

#[test]
fn invalid_compress_level_fails() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--compress-level", "foo"])
        .assert()
        .failure();
}

#[test]
fn out_of_range_compress_level_fails() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--compress-level", "10"])
        .assert()
        .failure();
}

#[test]
fn help_flag_prints_usage() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage:"));
}

#[cfg(unix)]
#[test]
fn links_preserve_symlinks() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("file"), b"hi").unwrap();
    symlink("file", src.join("link")).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = std::fs::symlink_metadata(dst.join("link")).unwrap();
    assert!(meta.file_type().is_symlink());
    let target = std::fs::read_link(dst.join("link")).unwrap();
    assert_eq!(target, std::path::PathBuf::from("file"));
}

#[cfg(unix)]
#[test]
fn links_copy_dangling_symlink() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    symlink("missing", src.join("dangling")).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = std::fs::symlink_metadata(dst.join("dangling")).unwrap();
    assert!(meta.file_type().is_symlink());
    let target = std::fs::read_link(dst.join("dangling")).unwrap();
    assert_eq!(target, std::path::PathBuf::from("missing"));
}

#[cfg(unix)]
#[test]
fn links_preserve_absolute_symlink() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    let file = src.join("file");
    std::fs::write(&file, b"hi").unwrap();
    let abs = file.canonicalize().unwrap();
    symlink(&abs, src.join("abs")).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let target = std::fs::read_link(dst.join("abs")).unwrap();
    assert_eq!(target, abs);
}

#[cfg(unix)]
#[test]
fn links_replace_and_skip_existing() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::write(src.join("file"), b"data").unwrap();
    symlink("file", src.join("link")).unwrap();
    std::fs::write(dst.join("link"), b"old").unwrap();

    let src_arg = format!("{}/", src.display());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();
    let meta = std::fs::symlink_metadata(dst.join("link")).unwrap();
    assert!(meta.file_type().is_symlink());
    std::fs::remove_file(dst.join("link")).unwrap();
    std::fs::write(dst.join("link"), b"old").unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--links",
            "--ignore-existing",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    let meta = std::fs::symlink_metadata(dst.join("link")).unwrap();
    assert!(!meta.file_type().is_symlink());
    let content = std::fs::read(dst.join("link")).unwrap();
    assert_eq!(content, b"old");
}
