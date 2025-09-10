// tests/misc_path.rs
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

#[test]
fn leading_dash_directory_requires_separator() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("-src");
    std::fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("sub/file.txt"), b"dash").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .current_dir(dir.path())
        .args(["-src/", "dst"])
        .assert()
        .failure();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .current_dir(dir.path())
        .args(["--", "-src/", "dst"])
        .assert()
        .success();

    assert_eq!(
        fs::read(dir.path().join("dst/sub/file.txt")).unwrap(),
        b"dash"
    );
}

#[test]
fn colon_in_path_triggers_remote_mode() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("dst")).unwrap();
    fs::write(dir.path().join("with:colon"), b"colon").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .current_dir(dir.path())
        .env("RSYNC_RSH", "false")
        .args(["with:colon", "dst"])
        .assert()
        .failure();
}

#[test]
fn relative_preserves_ancestors() {
    let dir = tempdir().unwrap();
    let src_root = dir.path().join("src");
    std::fs::create_dir_all(src_root.join("a/b")).unwrap();
    std::fs::write(src_root.join("a/b/file.txt"), b"hi").unwrap();
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.current_dir(dir.path());
    cmd.args(["-R", "src/a/b/", "dst"]);
    cmd.assert().success();

    let out = std::fs::read(dir.path().join("dst/src/a/b/file.txt")).unwrap();
    assert_eq!(out, b"hi");
}
