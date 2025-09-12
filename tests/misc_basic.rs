// tests/misc_basic.rs
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
fn prints_version() {
    let expected = oc_rsync_cli::version::version_banner();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(expected)
        .stderr("");
}

#[test]
fn remote_option_flag_accepts_multiple() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--remote-option=--log-file=/tmp/foo",
            "--remote-option=--fake-flag",
            "--version",
        ])
        .assert()
        .success();
}

#[test]
fn remote_option_short_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-M", "--log-file=/tmp/foo", "--version"])
        .assert()
        .success();
}

#[test]
fn iconv_invalid_charset_fails() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--iconv=FOO", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "iconv_open(\"FOO\", \"UTF-8\") failed",
        ));
}

#[test]
fn iconv_option_sent_to_daemon() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).unwrap();
        stream
            .write_all(&SUPPORTED_PROTOCOLS[0].to_be_bytes())
            .unwrap();
        let mut b = [0u8; 1];
        loop {
            stream.read_exact(&mut b).unwrap();
            if b[0] == b'\n' {
                break;
            }
        }
        stream.write_all(b"@RSYNCD: OK\n").unwrap();
        let mut line = Vec::new();
        loop {
            stream.read_exact(&mut b).unwrap();
            if b[0] == b'\n' {
                break;
            }
            line.push(b[0]);
        }
        assert_eq!(line, b"mod");
        let mut got = false;
        loop {
            line.clear();
            loop {
                stream.read_exact(&mut b).unwrap();
                if b[0] == b'\n' {
                    break;
                }
                line.push(b[0]);
            }
            if line.is_empty() {
                break;
            }
            if line == b"--iconv=utf8,latin1" {
                got = true;
            }
        }
        assert!(got);
    });

    let mut sync_opts = SyncOptions::default();
    sync_opts.remote_options.push("--iconv=utf8,latin1".into());
    let cv = parse_iconv("utf8,latin1").unwrap();
    let _ = spawn_daemon_session(
        "127.0.0.1",
        "mod",
        Some(port),
        None,
        true,
        None,
        None,
        None,
        &[],
        &sync_opts,
        SUPPORTED_PROTOCOLS[0],
        None,
        Some(&cv),
    )
    .unwrap();
    handle.join().unwrap();
}

#[test]
fn iconv_transcodes_filenames() {
    let spec = "utf8,latin1";
    let cv = parse_iconv(spec).unwrap();
    let remote = b"f\xF8o";
    let local = cv.to_local(remote).into_owned();
    assert_eq!(local, b"f\xC3\xB8o");
    let roundtrip = cv.to_remote(&local).into_owned();
    assert_eq!(roundtrip, remote);
}

#[test]
fn client_local_sync() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), b"hello world").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([&src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success().stdout("").stderr("");

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"hello world");
}

#[test]
fn whole_file_direct_copy() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let src_file = src_dir.join("a.txt");
    let dst_file = dst_dir.join("a.txt");
    std::fs::write(&src_file, b"new contents").unwrap();
    std::fs::write(&dst_file, b"old contents").unwrap();
    set_file_mtime(&dst_file, FileTime::from_unix_time(0, 0)).unwrap();

    std::fs::write(src_dir.join("b.txt"), b"brand new").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--whole-file", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"new contents");

    let out_new = std::fs::read(dst_dir.join("b.txt")).unwrap();
    assert_eq!(out_new, b"brand new");
}

#[test]
fn ignore_existing_skips_file() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    fs::write(src_dir.join("a.txt"), b"new").unwrap();
    let dst_file = dst_dir.join("a.txt");
    fs::write(&dst_file, b"old").unwrap();
    set_file_mtime(&dst_file, FileTime::from_unix_time(0, 0)).unwrap();
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--ignore-existing", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .success();
    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"old");
}

#[test]
fn existing_skips_new_file() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    fs::write(src_dir.join("a.txt"), b"new").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--existing", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .success();
    assert!(!dst_dir.join("a.txt").exists());
}

#[test]
fn existing_updates_existing_file() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    fs::write(src_dir.join("a.txt"), b"new").unwrap();
    let dst_file = dst_dir.join("a.txt");
    fs::write(&dst_file, b"old").unwrap();
    set_file_mtime(&dst_file, FileTime::from_unix_time(0, 0)).unwrap();
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--existing", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .success();
    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"new");
}

#[test]
fn size_only_skips_same_sized_file() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    fs::write(src_dir.join("a.txt"), b"new").unwrap();
    let dst_file = dst_dir.join("a.txt");
    fs::write(&dst_file, b"old").unwrap();
    set_file_mtime(&dst_file, FileTime::from_unix_time(0, 0)).unwrap();
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--size-only", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .success();
    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"old");
}

#[test]
fn ignore_times_forces_update() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let src_file = src_dir.join("a.txt");
    let dst_file = dst_dir.join("a.txt");
    fs::write(&src_file, b"new").unwrap();
    fs::write(&dst_file, b"old").unwrap();
    let t = FileTime::from_unix_time(1_000_000_000, 0);
    set_file_mtime(&src_file, t).unwrap();
    set_file_mtime(&dst_file, t).unwrap();
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--ignore-times", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .success();
    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"new");
}

#[test]
fn local_sync_without_flag_succeeds() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([&src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();
}

#[test]
fn resumes_from_partial_dir() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    let partial_dir = dir.path().join("partial");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), b"hello").unwrap();
    std::fs::create_dir_all(&partial_dir).unwrap();
    std::fs::write(partial_dir.join("a.txt"), b"he").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--partial",
        "--partial-dir",
        partial_dir.to_str().unwrap(),
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"hello");
    assert!(!partial_dir.join("a.txt").exists());
}

#[test]
fn resumes_from_partial_dir_with_absolute_path() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    let partial_dir = dir.path().join("partial");
    std::fs::create_dir_all(src_dir.join("sub")).unwrap();
    std::fs::write(src_dir.join("sub/a.txt"), b"hello").unwrap();
    std::fs::create_dir_all(&partial_dir).unwrap();
    std::fs::write(partial_dir.join("a.txt"), b"he").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--partial",
        "--partial-dir",
        partial_dir.to_str().unwrap(),
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("sub/a.txt")).unwrap();
    assert_eq!(out, b"hello");
    assert!(!partial_dir.join("a.txt").exists());
}
