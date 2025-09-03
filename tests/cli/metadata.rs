// tests/cli/metadata.rs

use assert_cmd::prelude::*;
use assert_cmd::Command;
use filetime::{set_file_mtime, FileTime};
use serial_test::serial;
use tempfile::tempdir;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use nix::sys::stat::{umask, Mode};

#[cfg(unix)]
#[test]
#[serial]
fn perms_flag_preserves_permissions() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("a.txt");
    fs::write(&file, b"hi").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o741)).unwrap();
    let dst_file = dst_dir.join("a.txt");
    fs::copy(&file, &dst_file).unwrap();
    fs::set_permissions(&dst_file, fs::Permissions::from_mode(0o600)).unwrap();
    let mtime = FileTime::from_last_modification_time(&fs::metadata(&file).unwrap());
    set_file_mtime(&dst_file, mtime).unwrap();

    let old_umask = umask(Mode::from_bits_truncate(0o077));

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--local", "--perms", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    umask(old_umask);

    let mode = fs::metadata(dst_dir.join("a.txt"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o7777, 0o741);
}

#[cfg(unix)]
#[test]
#[serial]
fn default_umask_masks_permissions() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("a.sh");
    fs::write(&file, b"hi").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o754)).unwrap();

    let old_umask = umask(Mode::from_bits_truncate(0o027));

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .success();

    umask(old_umask);

    let mode = fs::metadata(dst_dir.join("a.sh"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o754 & !0o027);
}

#[cfg(unix)]
#[test]
fn perms_preserve_permissions() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--perms", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.permissions().mode() & 0o7777, 0o640);
}

#[cfg(unix)]
#[test]
fn times_preserve_mtime() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let mtime = FileTime::from_unix_time(1_000_000, 0);
    set_file_mtime(&file, mtime).unwrap();
    let dst_file = dst.join("file");
    fs::copy(&file, &dst_file).unwrap();
    set_file_mtime(&dst_file, FileTime::from_unix_time(0, 0)).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--times", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = fs::metadata(dst.join("file")).unwrap();
    let dst_mtime = FileTime::from_last_modification_time(&meta);
    assert_eq!(dst_mtime, mtime);
}

