// tests/misc.rs
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
#[test]
#[cfg(unix)]
#[test]
#[cfg(unix)]
#[test]
#[cfg(unix)]
#[test]
#[cfg(unix)]
#[test]
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

#[cfg(unix)]
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

#[cfg(unix)]
#[test]
fn sparse_files_created() {
    use std::fs::File;
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    let zs = src.join("zeros");
    let mut f = File::create(&zs).unwrap();
    f.write_all(&vec![0u8; 1 << 18]).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--sparse", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let src_meta = std::fs::metadata(&zs).unwrap();
    if src_meta.blocks() * 512 >= src_meta.len() {
        eprintln!("skipping test: filesystem lacks sparse-file support");
        return;
    }
    let dst_meta = std::fs::metadata(dst.join("zeros")).unwrap();
    assert_eq!(src_meta.len(), dst_meta.len());
    if dst_meta.blocks() * 512 < dst_meta.len() {
        assert!(dst_meta.blocks() < src_meta.blocks());
    }
}

#[cfg(unix)]
#[test]
fn specials_preserve_fifo() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    let fifo = src.join("pipe");
    mkfifo(&fifo, nix::sys::stat::Mode::from_bits_truncate(0o600)).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--specials", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = std::fs::symlink_metadata(dst.join("pipe")).unwrap();
    assert!(meta.file_type().is_fifo());
}

#[test]
fn delete_delay_removes_extraneous_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::write(dst.join("old.txt"), b"old").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--delete-delay",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("old.txt").exists());
}

#[cfg(all(unix, feature = "xattr"))]
#[cfg(all(unix, feature = "xattr"))]
#[test]
fn super_overrides_fake_super() {
    if !Uid::effective().is_root() {
        eprintln!("skipping super_overrides_fake_super: requires root");
        return;
    }
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    let dst_dir = tmp.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    fs::write(src_dir.join("file"), b"hi").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-a",
            "--fake-super",
            "--super",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    let dst_file = dst_dir.join("file");
    assert!(xattr::get(&dst_file, "user.rsync.uid").unwrap().is_none());
}

#[test]
fn complex_glob_matches() {
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
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include",
            "dir*/**/keep[0-9].txt",
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
fn double_star_glob_matches() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("a/inner")).unwrap();
    fs::create_dir_all(src.join("b/deeper/more")).unwrap();
    fs::create_dir_all(src.join("c/other")).unwrap();
    fs::write(src.join("a/keep1.txt"), "1").unwrap();
    fs::write(src.join("a/inner/keep2.txt"), "2").unwrap();
    fs::write(src.join("b/deeper/more/keep3.txt"), "3").unwrap();
    fs::write(src.join("a/omit.log"), "x").unwrap();
    fs::write(src.join("a/inner/omit.log"), "x").unwrap();
    fs::write(src.join("b/deeper/more/omit.log"), "x").unwrap();
    fs::write(src.join("c/other/omit.log"), "x").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include",
            "**/keep?.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("a/keep1.txt").is_file());
    assert!(dst.join("a/inner/keep2.txt").is_file());
    assert!(dst.join("b/deeper/more/keep3.txt").is_file());
    assert!(dst.join("a").is_dir());
    assert!(dst.join("a/inner").is_dir());
    assert!(dst.join("b/deeper/more").is_dir());
    assert!(!dst.join("a/omit.log").exists());
    assert!(!dst.join("a/inner/omit.log").exists());
    assert!(!dst.join("b/deeper/more/omit.log").exists());
    assert!(!dst.join("c/other/omit.log").exists());
    assert!(!dst.join("c").exists());
}

#[test]
fn single_star_does_not_cross_directories() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("a/b")).unwrap();
    fs::write(src.join("a/file.txt"), b"1").unwrap();
    fs::write(src.join("a/b/file.txt"), b"2").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include",
            "*/file.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("a/file.txt").exists());
    assert!(dst.join("a").is_dir());
    assert!(!dst.join("a/b/file.txt").exists());
    assert!(!dst.join("a/b").exists());
}

#[test]
fn segment_star_does_not_cross_directories() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("data/sub")).unwrap();
    fs::write(src.join("data/file.txt"), b"1").unwrap();
    fs::write(src.join("data/sub/file.txt"), b"2").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include",
            "data*/file.txt",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("data/file.txt").exists());
    assert!(dst.join("data").is_dir());
    assert!(!dst.join("data/sub/file.txt").exists());
    assert!(!dst.join("data/sub").exists());
}

#[test]
fn char_class_respects_directory_boundaries() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("1/2")).unwrap();
    fs::write(src.join("1/keep.txt"), b"k").unwrap();
    fs::write(src.join("1/2/keep.txt"), b"x").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            "--include",
            "[0-9]/*",
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("1/keep.txt").exists());
    assert!(dst.join("1").is_dir());
    assert!(dst.join("1/2").is_dir());
    assert!(!dst.join("1/2/keep.txt").exists());
}
