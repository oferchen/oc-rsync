// tests/files_from_extra.rs
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
mod util;
use util::setup_files_from_env;

#[test]
fn files_from_zero_separated_list_with_crlf() {
    let (dir, src, dst) = setup_files_from_env(&[("keep me.txt", b"k"), ("skip.txt", b"s")]);
    let list = dir.path().join("files.lst");
    std::fs::write(&list, b"keep me.txt\r\n\0").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--files-from",
            list.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("keep me.txt").exists());
    assert!(!dst.join("skip.txt").exists());
}
#[test]
fn files_from_zero_separated_list_allows_hash() {
    let (dir, src, dst) = setup_files_from_env(&[("#keep.txt", b"k"), ("skip.txt", b"s")]);
    let list = dir.path().join("files.lst");
    std::fs::write(&list, b"#keep.txt\0").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--files-from",
            list.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("#keep.txt").exists());
    assert!(!dst.join("skip.txt").exists());
}
#[test]
fn files_from_zero_separated_list_includes_directories() {
    let (dir, src, dst) = setup_files_from_env(&[("dir/sub/file.txt", b"k")]);
    let list = dir.path().join("files.lst");
    std::fs::write(&list, b"dir\0").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--files-from",
            list.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("dir/sub/file.txt").exists());
}
#[test]
fn files_from_single_file_creates_implied_directories() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let oc_dst = dir.path().join("oc");
    fs::create_dir_all(src.join("foo/bar")).unwrap();
    fs::write(src.join("foo/bar/baz.txt"), b"k").unwrap();
    fs::create_dir_all(&oc_dst).unwrap();
    let list = dir.path().join("files.lst");
    fs::write(&list, "foo/bar/baz.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());
    let out = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-n",
            "--recursive",
            "--files-from",
            list.to_str().unwrap(),
            "--out-format=%n",
            &src_arg,
            oc_dst.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let paths: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();
    assert_eq!(paths, vec!["foo/", "foo/bar/", "foo/bar/baz.txt"]);

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--files-from",
            list.to_str().unwrap(),
            &src_arg,
            oc_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(oc_dst.join("foo").is_dir());
    assert!(oc_dst.join("foo/bar").is_dir());
    assert!(oc_dst.join("foo/bar/baz.txt").is_file());

    let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/files_from/single_file_creates_implied_dirs");
    let diff = std::process::Command::new("diff")
        .arg("-r")
        .arg(&golden)
        .arg(&oc_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}
#[test]
fn files_from_single_file_no_implied_dirs_fails_like_rsync() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let oc_dst = dir.path().join("oc");
    fs::create_dir_all(src.join("foo/bar")).unwrap();
    fs::write(src.join("foo/bar/baz.txt"), b"k").unwrap();
    fs::create_dir_all(&oc_dst).unwrap();
    let list = dir.path().join("files.lst");
    fs::write(&list, "foo/bar/baz.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());
    let (_exp_stdout, _exp_stderr, exp_exit) = read_golden("files_from/no_implied_dirs");

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--no-implied-dirs",
            "--files-from",
            list.to_str().unwrap(),
            &src_arg,
            oc_dst.to_str().unwrap(),
        ])
        .assert()
        .code(exp_exit);

    assert!(fs::read_dir(&oc_dst).unwrap().next().is_none());
}
#[test]
fn files_from_zero_separated_list_directory_without_slash_excludes_siblings() {
    let (dir, src, dst) =
        setup_files_from_env(&[("dir/sub/file.txt", b"k"), ("other/file.txt", b"o")]);
    let list = dir.path().join("files.lst");
    fs::write(&list, b"dir\0").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--files-from",
            list.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("dir/sub/file.txt").exists());
    assert!(!dst.join("other/file.txt").exists());
}
#[test]
fn files_from_list_file() {
    let (dir, src, dst) = setup_files_from_env(&[("keep.txt", b"k"), ("skip.txt", b"s")]);
    let list = dir.path().join("files.lst");
    std::fs::write(&list, "# comment\nkeep.txt\n\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--files-from",
            list.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("skip.txt").exists());
}
#[test]
fn files_from_list_handles_crlf_and_comment_spaces() {
    let (dir, src, dst) = setup_files_from_env(&[("keep.txt", b"k"), ("skip.txt", b"s")]);
    let list = dir.path().join("files.lst");
    std::fs::write(&list, "  # comment\r\nkeep.txt\r\n\r\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--files-from",
            list.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("skip.txt").exists());
}
#[test]
fn files_from_list_includes_directories() {
    let (dir, src, dst) = setup_files_from_env(&[("dir/sub/file.txt", b"k")]);
    let list = dir.path().join("files.lst");
    std::fs::write(&list, "dir\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--files-from",
            list.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("dir/sub/file.txt").exists());
}
#[test]
fn files_from_from0_trailing_slash_semantics() {
    let (tmp, src, dst) =
        setup_files_from_env(&[("dir/sub/file.txt", b"k"), ("dir/other.txt", b"o")]);

    let list = tmp.path().join("files.lst");
    fs::write(&list, b"dir\0dir/sub/file.txt\0").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--files-from",
            list.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        dst.join("dir/sub/file.txt").is_file(),
        "path {}",
        dst.join("dir/sub/file.txt").display()
    );
    assert!(
        !dst.join("dir/other.txt").exists(),
        "path {}",
        dst.join("dir/other.txt").display()
    );

    let dst2 = tmp.path().join("dst2");
    fs::create_dir_all(&dst2).unwrap();
    let list2 = tmp.path().join("files2.lst");
    fs::write(&list2, b"dir/\0").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--from0",
            "--files-from",
            list2.to_str().unwrap(),
            &src_arg,
            dst2.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        dst2.join("dir/sub/file.txt").is_file(),
        "path {}",
        dst2.join("dir/sub/file.txt").display()
    );
    assert!(
        dst2.join("dir/other.txt").is_file(),
        "path {}",
        dst2.join("dir/other.txt").display()
    );
}
