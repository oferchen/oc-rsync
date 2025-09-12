// tests/misc_symlinks.rs
#![allow(unused_imports)]

use assert_cmd::prelude::*;
use assert_cmd::{cargo::cargo_bin, Command};
use engine::SyncOptions;
use filetime::{set_file_mtime, FileTime};
#[cfg(unix)]
use nix::unistd::{chown, mkfifo, Gid, Uid};
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
use tempfile::{tempdir, tempdir_in, TempDir};
#[cfg(unix)]
use users::{get_current_gid, get_current_uid, get_group_by_gid, get_user_by_uid};
mod common;
use common::read_golden;

#[allow(clippy::vec_init_then_push)]
#[allow(clippy::vec_init_then_push)]
#[test]
fn links_preserve_directory_symlinks() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(src.join("dir")).unwrap();
    std::fs::write(src.join("dir/file"), b"hi").unwrap();
    symlink("dir", src.join("dirlink")).unwrap();

    std::fs::create_dir_all(&dst).unwrap();
    symlink("dir", dst.join("dirlink")).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = std::fs::symlink_metadata(dst.join("dirlink")).unwrap();
    assert!(meta.file_type().is_symlink());
    let target = std::fs::read_link(dst.join("dirlink")).unwrap();
    assert_eq!(target, std::path::PathBuf::from("dir"));
}

#[cfg(unix)]
#[test]
fn copy_dirlinks_transforms_directory_symlinks() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(src.join("dir")).unwrap();
    std::fs::write(src.join("dir/file"), b"hi").unwrap();
    std::fs::write(src.join("file"), b"data").unwrap();
    symlink("dir", src.join("dirlink")).unwrap();
    symlink("file", src.join("filelink")).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--links",
            "--copy-dirlinks",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("dirlink").is_dir());
    let meta = std::fs::symlink_metadata(dst.join("filelink")).unwrap();
    assert!(meta.file_type().is_symlink());
}

#[cfg(unix)]
#[test]
fn keep_dirlinks_handles_nested_symlinks() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    let target = dir.path().join("target");
    let nested = dir.path().join("nested");

    std::fs::create_dir_all(src.join("a/b")).unwrap();
    std::fs::write(src.join("a/b/file"), b"hi").unwrap();

    std::fs::create_dir_all(&dst).unwrap();
    std::fs::create_dir_all(&target).unwrap();
    symlink(&target, dst.join("a")).unwrap();
    symlink(&nested, target.join("b")).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--keep-dirlinks", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = std::fs::symlink_metadata(dst.join("a")).unwrap();
    assert!(meta.file_type().is_symlink());
    let nested_meta = std::fs::symlink_metadata(target.join("b")).unwrap();
    assert!(nested_meta.file_type().is_symlink());
    assert!(nested.join("file").exists());
}

#[cfg(unix)]
#[test]
fn copy_links_resolves_relative_and_absolute_targets() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    let file = src.join("file");
    std::fs::write(&file, b"data").unwrap();
    symlink("file", src.join("rel")).unwrap();
    let abs = file.canonicalize().unwrap();
    symlink(&abs, src.join("abs")).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--copy-links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    assert!(dst.join("rel").is_file());
    assert_eq!(std::fs::read(dst.join("rel")).unwrap(), b"data");
    assert!(dst.join("abs").is_file());
    assert_eq!(std::fs::read(dst.join("abs")).unwrap(), b"data");
}

#[cfg(unix)]
#[test]
fn copy_links_errors_on_dangling_symlink() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    symlink("missing", src.join("dangling")).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--copy-links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("symlink has no referent"));
    assert!(!dst.join("dangling").exists());
}

#[cfg(unix)]
#[test]
fn safe_links_resolve_source_symlink() {
    let dir = tempdir().unwrap();
    let real = dir.path().join("real");
    let link = dir.path().join("link");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&real).unwrap();
    std::fs::write(real.join("file"), b"hi").unwrap();
    symlink("file", real.join("safe")).unwrap();
    symlink("real", &link).unwrap();

    let src_arg = format!("{}/", link.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--links", "--safe-links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = std::fs::symlink_metadata(dst.join("safe")).unwrap();
    assert!(meta.file_type().is_symlink());
}

#[cfg(unix)]
#[test]
fn safe_links_skip_absolute_symlink() {
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
        .args(["--links", "--safe-links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    assert!(!dst.join("abs").exists());
}

#[cfg(unix)]
#[test]
fn safe_links_allow_relative_symlink_within_tree() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(src.join("dir")).unwrap();
    std::fs::write(src.join("file"), b"hi").unwrap();
    symlink("../file", src.join("dir/link")).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--links", "--safe-links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = std::fs::symlink_metadata(dst.join("dir/link")).unwrap();
    assert!(meta.file_type().is_symlink());
    let target = std::fs::read_link(dst.join("dir/link")).unwrap();
    assert_eq!(target, std::path::PathBuf::from("../file"));
}

#[cfg(unix)]
#[test]
fn perms_preserve_permissions() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    let file = src.join("file");
    std::fs::write(&file, b"hi").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--perms", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = std::fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.permissions().mode() & 0o7777, 0o640);
}

#[cfg(unix)]
#[test]
fn times_preserve_mtime() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    std::fs::write(&file, b"hi").unwrap();
    let mtime = FileTime::from_unix_time(1_000_000, 0);
    set_file_mtime(&file, mtime).unwrap();
    let dst_file = dst.join("file");
    std::fs::copy(&file, &dst_file).unwrap();
    set_file_mtime(&dst_file, FileTime::from_unix_time(0, 0)).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--times", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = std::fs::metadata(dst.join("file")).unwrap();
    let dst_mtime = FileTime::from_last_modification_time(&meta);
    assert_eq!(dst_mtime, mtime);
}

#[cfg(unix)]
#[test]
fn sparse_files_preserved() {
    use std::fs::File;
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    let sp = src.join("sparse");
    let mut f = File::create(&sp).unwrap();
    f.seek(SeekFrom::Start(1 << 17)).unwrap();
    f.write_all(b"end").unwrap();
    f.set_len(1 << 18).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--sparse", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let src_meta = std::fs::metadata(&sp).unwrap();
    if src_meta.blocks() * 512 >= src_meta.len() {
        eprintln!("skipping test: filesystem lacks sparse-file support");
        return;
    }
    let dst_meta = std::fs::metadata(dst.join("sparse")).unwrap();
    assert_eq!(src_meta.len(), dst_meta.len());
    assert_eq!(src_meta.blocks(), dst_meta.blocks());
    if src_meta.blocks() * 512 < src_meta.len() {
        assert!(dst_meta.blocks() * 512 < dst_meta.len());
    }
}
