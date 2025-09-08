// tests/filter.rs
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
