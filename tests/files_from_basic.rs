// tests/files_from_basic.rs
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
fn files_from_from0_matches_rsync() {
    let (tmp, src, ours_dst) =
        setup_files_from_env(&[("include_me.txt", b"hi"), ("omit.log", b"nope")]);
    let rsync_dst = tmp.path().join("rsync");
    fs::create_dir_all(&rsync_dst).unwrap();

    let list = tmp.path().join("list");
    fs::write(&list, b"include_me.txt\0omit.log\0").unwrap();

    let src_arg = format!("{}/", src.display());
    let status = std::process::Command::new(cargo_bin("oc-rsync"))
        .args([
            "-r",
            "--from0",
            "--files-from",
            list.to_str().unwrap(),
            "--exclude",
            "*.log",
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
            "--files-from",
            list.to_str().unwrap(),
            "--exclude",
            "*.log",
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
fn files_from_nested_file_creates_parent_dirs() {
    let (tmp, src, dst) = setup_files_from_env(&[
        ("foo/bar/baz.txt", b"data"),
        ("foo/bar/skip.txt", b"no"),
        ("foo/other.txt", b"no"),
        ("qux/sub/keep.txt", b"data"),
        ("qux/junk.txt", b"data"),
        ("other/file.txt", b"no"),
    ]);
    let list = tmp.path().join("list");
    fs::write(&list, "foo/bar/baz.txt\n").unwrap();
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
    assert!(
        dst.join("foo").is_dir(),
        "path {}",
        dst.join("foo").display()
    );
    assert!(
        dst.join("foo/bar").is_dir(),
        "path {}",
        dst.join("foo/bar").display()
    );
    assert!(
        dst.join("foo/bar/baz.txt").is_file(),
        "path {}",
        dst.join("foo/bar/baz.txt").display()
    );
    assert!(
        !dst.join("foo/bar/skip.txt").exists(),
        "path {}",
        dst.join("foo/bar/skip.txt").display()
    );
    assert!(
        !dst.join("foo/other.txt").exists(),
        "path {}",
        dst.join("foo/other.txt").display()
    );
    assert!(
        !dst.join("qux").exists(),
        "path {}",
        dst.join("qux").display()
    );
    assert!(
        !dst.join("other").exists(),
        "path {}",
        dst.join("other").display()
    );
}
#[test]
fn files_from_directory_copies_entire_tree() {
    let (tmp, src, dst) = setup_files_from_env(&[
        ("foo/bar/baz.txt", b"data"),
        ("foo/bar/skip.txt", b"no"),
        ("foo/other.txt", b"no"),
        ("qux/sub/keep.txt", b"data"),
        ("qux/junk.txt", b"data"),
        ("other/file.txt", b"no"),
    ]);
    let list = tmp.path().join("list");
    fs::write(&list, "qux/\n").unwrap();
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
    assert!(
        dst.join("qux").is_dir(),
        "path {}",
        dst.join("qux").display()
    );
    assert!(
        dst.join("qux/sub").is_dir(),
        "path {}",
        dst.join("qux/sub").display()
    );
    assert!(
        dst.join("qux/sub/keep.txt").is_file(),
        "path {}",
        dst.join("qux/sub/keep.txt").display()
    );
    assert!(
        dst.join("qux/junk.txt").is_file(),
        "path {}",
        dst.join("qux/junk.txt").display()
    );
    assert!(
        !dst.join("foo").exists(),
        "path {}",
        dst.join("foo").display()
    );
    assert!(
        !dst.join("other").exists(),
        "path {}",
        dst.join("other").display()
    );
}
#[test]
fn files_from_dirs_and_nested_paths_from0() {
    let (tmp, src, dst) = setup_files_from_env(&[
        ("foo/bar/baz.txt", b"data"),
        ("foo/bar/skip.txt", b"no"),
        ("foo/other.txt", b"no"),
        ("qux/sub/keep.txt", b"data"),
        ("qux/junk.txt", b"data"),
        ("other/file.txt", b"no"),
    ]);
    let list = tmp.path().join("list");
    fs::write(&list, b"foo/bar/baz.txt\0qux/\0").unwrap();
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
    assert!(dst.join("foo").is_dir());
    assert!(dst.join("foo/bar").is_dir());
    assert!(dst.join("foo/bar/baz.txt").is_file());
    assert!(!dst.join("foo/bar/skip.txt").exists());
    assert!(!dst.join("foo/other.txt").exists());
    assert!(dst.join("qux").is_dir());
    assert!(dst.join("qux/sub").is_dir());
    assert!(dst.join("qux/sub/keep.txt").is_file());
    assert!(dst.join("qux/junk.txt").is_file());
    assert!(!dst.join("other").exists());
}
#[test]
fn files_from_list_transfers_only_listed_files() {
    let (dir, src, dst) = setup_files_from_env(&[
        ("keep.txt", b"k"),
        ("other file.txt", b"o"),
        ("skip.txt", b"s"),
    ]);
    let list = dir.path().join("files.txt");
    fs::write(&list, "keep.txt\nother\\ file.txt\n#comment\n").unwrap();

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
    assert!(dst.join("other file.txt").exists());
    assert!(!dst.join("skip.txt").exists());
}
#[test]
fn files_from_list_transfers_nested_paths() {
    let (dir, src, dst) = setup_files_from_env(&[
        ("a/b/file.txt", b"f"),
        ("a/b/other.txt", b"o"),
        ("a/d/sub/nested.txt", b"n"),
        ("unlisted.txt", b"u"),
    ]);
    let list = dir.path().join("files.txt");
    fs::write(&list, "a/b/file.txt\na/d/\n").unwrap();

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

    assert!(dst.join("a/b/file.txt").exists());
    assert!(dst.join("a/d/sub/nested.txt").exists());
    assert!(!dst.join("a/b/other.txt").exists());
    assert!(!dst.join("unlisted.txt").exists());
}
#[test]
fn files_from_list_nested_file_excludes_siblings() {
    let (dir, src, dst) = setup_files_from_env(&[
        ("a/b/file.txt", b"f"),
        ("a/b/other.txt", b"o"),
        ("a/c/unrelated.txt", b"u"),
    ]);
    let list = dir.path().join("files.txt");
    fs::write(&list, "a/b/file.txt\n").unwrap();

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

    assert!(dst.join("a/b/file.txt").exists());
    assert!(!dst.join("a/b/other.txt").exists());
    assert!(!dst.join("a/c/unrelated.txt").exists());
}
#[test]
fn files_from_list_directory_excludes_siblings() {
    let (dir, src, dst) =
        setup_files_from_env(&[("dir/sub/file.txt", b"k"), ("other/file.txt", b"o")]);
    let list = dir.path().join("files.txt");
    fs::write(&list, "dir/\n").unwrap();

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
    assert!(!dst.join("other/file.txt").exists());
}
#[test]
fn files_from_list_directory_without_slash_excludes_siblings() {
    let (dir, src, dst) =
        setup_files_from_env(&[("dir/sub/file.txt", b"k"), ("other/file.txt", b"o")]);
    let list = dir.path().join("files.txt");
    fs::write(&list, "dir\n").unwrap();

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
    assert!(!dst.join("other/file.txt").exists());
}
#[test]
fn files_from_zero_separated_list() {
    let (dir, src, dst) = setup_files_from_env(&[("keep me.txt", b"k"), ("skip.txt", b"s")]);
    let list = dir.path().join("files.lst");
    std::fs::write(&list, b"keep me.txt\0").unwrap();

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
