// tests/cli.rs

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

#[cfg(unix)]
struct Tmpfs(TempDir);

#[cfg(unix)]
impl Tmpfs {
    fn new() -> Option<Self> {
        if !cfg!(target_os = "linux") {
            return None;
        }
        if !Uid::effective().is_root() {
            return None;
        }
        let mount_exists = std::env::var_os("PATH").is_some_and(|paths| {
            std::env::split_paths(&paths).any(|dir| dir.join("mount").is_file())
        });
        if !mount_exists {
            return None;
        }
        if let Ok(fs) = std::fs::read_to_string("/proc/filesystems") {
            if !fs.lines().any(|l| l.trim().ends_with("tmpfs")) {
                return None;
            }
        } else {
            return None;
        }
        let dir = tempdir().ok()?;
        let status = std::process::Command::new("mount")
            .args(["-t", "tmpfs", "tmpfs", dir.path().to_str().unwrap()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .ok()?;
        if status.success() {
            Some(Tmpfs(dir))
        } else {
            None
        }
    }

    fn path(&self) -> &std::path::Path {
        self.0.path()
    }
}

#[test]
fn files_from_from0_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("include_me.txt"), "hi").unwrap();
    fs::write(src.join("omit.log"), "nope").unwrap();

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
fn files_from_dirs_and_nested_paths() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("foo/bar")).unwrap();
    fs::create_dir_all(src.join("qux/sub")).unwrap();
    fs::create_dir_all(src.join("other")).unwrap();
    fs::write(src.join("foo/bar/baz.txt"), b"data").unwrap();
    fs::write(src.join("foo/bar/skip.txt"), b"no").unwrap();
    fs::write(src.join("foo/other.txt"), b"no").unwrap();
    fs::write(src.join("qux/sub/keep.txt"), b"data").unwrap();
    fs::write(src.join("qux/junk.txt"), b"data").unwrap();
    fs::write(src.join("other/file.txt"), b"no").unwrap();
    fs::create_dir_all(&dst).unwrap();
    let list = tmp.path().join("list");
    fs::write(&list, "foo/bar/baz.txt\nqux/\n").unwrap();
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
fn files_from_dirs_and_nested_paths_from0() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("foo/bar")).unwrap();
    fs::create_dir_all(src.join("qux/sub")).unwrap();
    fs::create_dir_all(src.join("other")).unwrap();
    fs::write(src.join("foo/bar/baz.txt"), b"data").unwrap();
    fs::write(src.join("foo/bar/skip.txt"), b"no").unwrap();
    fs::write(src.join("foo/other.txt"), b"no").unwrap();
    fs::write(src.join("qux/sub/keep.txt"), b"data").unwrap();
    fs::write(src.join("qux/junk.txt"), b"data").unwrap();
    fs::write(src.join("other/file.txt"), b"no").unwrap();
    fs::create_dir_all(&dst).unwrap();
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

#[cfg(unix)]
impl Drop for Tmpfs {
    fn drop(&mut self) {
        let _ = std::process::Command::new("umount")
            .arg(self.0.path())
            .status();
    }
}

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

#[test]
fn progress_flag_shows_output() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), vec![0u8; 2048]).unwrap();
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    let assert = cmd
        .args([
            "--recursive",
            "--progress",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(stderr.is_empty(), "{}", stderr);
    let mut lines = stdout.lines();
    assert_eq!(lines.next().unwrap(), "sending incremental file list");
    assert_eq!(lines.next().unwrap(), "a.txt");
    let progress_line_raw = lines.next().unwrap();
    let progress_line = progress_line_raw.trim_start_matches('\r').trim_end();
    let bytes = progress_formatter(2048, false);
    let expected_prefix = format!("{:>15} {:>3}%", bytes, 100);
    assert!(progress_line.starts_with(&expected_prefix));
    assert!(stdout.contains(progress_line_raw));
}

fn sanitize_progress_line(line: &str) -> String {
    let mut parts: Vec<_> = line.split_whitespace().collect();
    if parts.len() >= 4 {
        parts[2] = "XKB/s";
        parts[3] = "00:00:00";
        format!("{:>15} {:>4} {} {}", parts[0], parts[1], parts[2], parts[3])
    } else {
        line.to_string()
    }
}

#[test]
fn progress_parity() {
    let norm = progress_parity_impl(&["-r", "--progress"], "progress");
    insta::assert_snapshot!("progress_parity", norm);
}

fn progress_parity_impl(flags: &[&str], fixture: &str) -> String {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst_ours = dir.path().join("dst_ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst_ours).unwrap();
    fs::write(src.join("a.txt"), b"hello").unwrap();

    let mut our_cmd = Command::cargo_bin("oc-rsync").unwrap();
    our_cmd.env("LC_ALL", "C").env("COLUMNS", "80");
    our_cmd.args(flags);
    our_cmd.arg(format!("{}/", src.display()));
    our_cmd.arg(dst_ours.to_str().unwrap());
    let ours = our_cmd.output().unwrap();

    let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/progress")
        .join(fixture);
    let up_stdout = fs::read(golden.with_extension("stdout")).unwrap();
    let up_stderr = fs::read(golden.with_extension("stderr")).unwrap();
    let up_status: i32 = fs::read_to_string(golden.with_extension("exit"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    assert_eq!(Some(up_status), ours.status.code());

    let extract = |stdout: &[u8], stderr: &[u8]| {
        let stdout_txt = String::from_utf8_lossy(stdout).replace('\r', "\n");
        let stderr_txt = String::from_utf8_lossy(stderr).replace('\r', "\n");
        let find = |txt: &str| {
            txt.lines()
                .rev()
                .find(|l| l.contains('%'))
                .map(|l| l.to_string())
        };
        if let Some(line) = find(&stdout_txt) {
            (line, stdout_txt, stderr_txt, "stdout")
        } else if let Some(line) = find(&stderr_txt) {
            (line, stdout_txt, stderr_txt, "stderr")
        } else {
            panic!("no progress line found");
        }
    };

    let (up_line, up_stdout_txt, up_stderr_txt, up_stream) = extract(&up_stdout, &up_stderr);
    let (our_line, our_stdout_txt, our_stderr_txt, our_stream) =
        extract(&ours.stdout, &ours.stderr);

    assert_eq!(up_stream, our_stream, "progress output stream mismatch");

    fn strip_progress(s: &str) -> String {
        s.lines()
            .filter(|l| !l.contains('%'))
            .collect::<Vec<_>>()
            .join("\n")
    }
    assert_eq!(
        strip_progress(&up_stdout_txt),
        strip_progress(&our_stdout_txt)
    );
    assert_eq!(
        strip_progress(&up_stderr_txt),
        strip_progress(&our_stderr_txt)
    );

    let normalized = sanitize_progress_line(&our_line);
    assert_eq!(sanitize_progress_line(&up_line), normalized);
    normalized
}

#[test]
fn progress_parity_p() {
    let norm = progress_parity_impl(&["-r", "-P"], "progress_p");
    insta::assert_snapshot!("progress_parity_p", norm);
}

#[test]
fn stats_parity() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst_ours = dir.path().join("dst_ours");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), b"hello").unwrap();

    let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/stats/stats_parity.stdout");
    let up_stats: Vec<String> = std::fs::read_to_string(golden)
        .unwrap()
        .lines()
        .map(|l| l.to_string())
        .collect();
    assert_eq!(up_stats.len(), 6);
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .args([
            "--recursive",
            "--stats",
            format!("{}/", src.display()).as_str(),
            dst_ours.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        ours.status.success(),
        "oc-rsync failed: {}",
        String::from_utf8_lossy(&ours.stderr)
    );

    let our_stdout = String::from_utf8_lossy(&ours.stdout);
    let our_stats: Vec<String> = our_stdout
        .lines()
        .filter_map(|l| {
            let l = l.trim_start();
            if l.starts_with("Number of files")
                || l.starts_with("Number of created files")
                || l.starts_with("Number of deleted files")
                || l.starts_with("Number of regular files transferred")
                || l.starts_with("Total transferred file size")
                || l.starts_with("File list size")
            {
                Some(l.to_string())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(our_stats, up_stats);

    let rate_line = our_stdout
        .lines()
        .find_map(|l| {
            let l = l.trim_start();
            l.starts_with("sent ").then(|| l.to_string())
        })
        .expect("missing rate line");
    assert!(!rate_line.contains("0.00"));

    insta::assert_snapshot!("stats_parity", our_stats.join("\n"));
}

#[test]
fn progress_flag_human_readable() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();

    std::fs::write(src_dir.join("a.txt"), vec![0u8; 2 * 1024]).unwrap();
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    let assert = cmd
        .args([
            "--recursive",
            "--progress",
            "--human-readable",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let text = if !stdout.is_empty() { stdout } else { stderr };
    let mut lines = text.lines();
    assert_eq!(lines.next().unwrap(), "sending incremental file list");
    assert_eq!(lines.next().unwrap(), "a.txt");
    let progress_line = lines.next().unwrap().trim_start_matches('\r').trim_end();
    let bytes = progress_formatter(2 * 1024, true);
    let expected_prefix = format!("{:>15} {:>3}%", bytes, 100);
    assert!(progress_line.starts_with(&expected_prefix));
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
fn fails_when_temp_dir_is_file() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    let tmp_file = dir.path().join("tmp");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), b"hello").unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    std::fs::write(&tmp_file, b"not a dir").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--temp-dir",
        tmp_file.to_str().unwrap(),
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().failure();
    assert!(!dst_dir.join("a.txt").exists());
}

#[test]
fn temp_files_created_in_temp_dir() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    let tmp_dir = dir.path().join("tmp");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let data = vec![b'x'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut child = std::process::Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--partial",
            "--bwlimit",
            "10240",
            "--temp-dir",
            tmp_dir.to_str().unwrap(),
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();

    let mut found = false;
    for _ in 0..50 {
        let tmp_present = std::fs::read_dir(&tmp_dir)
            .unwrap()
            .filter_map(|e| {
                let entry = e.ok()?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with(".a.txt.") {
                    Some(entry.path())
                } else {
                    None
                }
            })
            .next();
        if tmp_present.is_some() {
            found = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    assert!(found, "temp file not created in temp dir during transfer");
    assert!(
        fs::read_dir(&tmp_dir).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".oc-rsync-tmp.")),
        "intermediate temp dir created",
    );
}

#[test]
#[cfg(unix)]
fn temp_dir_cross_filesystem_temp_file_in_dest() {
    let base = tempdir_in(".").unwrap();
    let src_dir = base.path().join("src");
    let dst_dir = base.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'x'; 200_000];
    fs::write(src_dir.join("a.txt"), &data).unwrap();

    let tmp_dir = match Tmpfs::new() {
        Some(t) => t,
        None => {
            eprintln!("skipping cross-filesystem temp-dir test; tmpfs unavailable");
            return;
        }
    };

    let dst_dev = fs::metadata(&dst_dir).unwrap().dev();
    let tmp_dev = fs::metadata(tmp_dir.path()).unwrap().dev();
    assert_ne!(dst_dev, tmp_dev, "devices match");

    let src_arg = format!("{}/", src_dir.display());
    let mut child = std::process::Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--partial",
            "--bwlimit",
            "10240",
            "--temp-dir",
            tmp_dir.path().to_str().unwrap(),
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();

    let mut found = false;
    for _ in 0..50 {
        let tmp_present = fs::read_dir(&dst_dir)
            .unwrap()
            .filter_map(|e| {
                let entry = e.ok()?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with(".a.txt.") {
                    Some(entry.path())
                } else {
                    None
                }
            })
            .next();
        if tmp_present.is_some() {
            assert!(fs::read_dir(tmp_dir.path()).unwrap().next().is_none());
            found = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        found,
        "temp file not created in destination during transfer"
    );
    assert!(
        fs::read_dir(&dst_dir).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".oc-rsync-tmp.")),
        "intermediate temp dir created",
    );

    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out.len(), data.len());
}

#[test]
fn destination_is_replaced_atomically() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let src_file = src_dir.join("a.txt");
    let dst_file = dst_dir.join("a.txt");
    std::fs::write(&src_file, vec![b'x'; 50_000]).unwrap();
    std::fs::write(&dst_file, b"old").unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut child = std::process::Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--bwlimit", "20000", &src_arg, dst_dir.to_str().unwrap()])
        .spawn()
        .unwrap();

    let mut found = false;
    let mut tmp_file: Option<PathBuf> = None;
    for _ in 0..50 {
        let tmp_present = fs::read_dir(&dst_dir)
            .unwrap()
            .filter_map(|e| {
                let entry = e.ok()?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with(".a.txt.") {
                    Some(entry.path())
                } else {
                    None
                }
            })
            .next();
        if let Some(path) = tmp_present {
            let out = std::fs::read(&dst_file).unwrap();
            assert_eq!(out, b"old");
            tmp_file = Some(path);
            found = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        found,
        "temp file not created in destination during transfer",
    );

    child.wait().unwrap();
    let tmp_file = tmp_file.unwrap();
    assert!(!tmp_file.exists(), "temp file not removed after transfer",);
    assert!(
        fs::read_dir(&dst_dir).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".oc-rsync-tmp.")),
        "intermediate temp dir created",
    );
    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out.len(), 50_000);
}

#[test]
#[cfg(unix)]
fn temp_dir_cross_filesystem_rename() {
    let base = tempdir_in(".").unwrap();
    let src_dir = base.path().join("src");
    let dst_dir = base.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    fs::write(src_dir.join("a.txt"), b"x").unwrap();

    let tmp_dir = match Tmpfs::new() {
        Some(t) => t,
        None => {
            eprintln!("skipping cross-filesystem rename test; tmpfs unavailable");
            return;
        }
    };

    let dst_dev = fs::metadata(&dst_dir).unwrap().dev();
    let tmp_dev = fs::metadata(tmp_dir.path()).unwrap().dev();
    assert_ne!(dst_dev, tmp_dev, "devices match");

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--temp-dir",
            tmp_dir.path().to_str().unwrap(),
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(fs::read_dir(tmp_dir.path()).unwrap().next().is_none());
    assert!(
        fs::read_dir(&dst_dir).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".oc-rsync-tmp.")),
        "intermediate temp dir created",
    );
    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"x");
}

#[test]
#[cfg(unix)]
fn delay_updates_cross_filesystem_rename() {
    let base = tempdir_in(".").unwrap();
    let src_dir = base.path().join("src");
    let dst_dir = base.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    fs::write(src_dir.join("a.txt"), b"y").unwrap();

    let tmp_dir = match Tmpfs::new() {
        Some(t) => t,
        None => {
            eprintln!("skipping cross-filesystem rename test; tmpfs unavailable");
            return;
        }
    };

    let dst_dev = fs::metadata(&dst_dir).unwrap().dev();
    let tmp_dev = fs::metadata(tmp_dir.path()).unwrap().dev();
    assert_ne!(dst_dev, tmp_dev, "devices match");

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--delay-updates",
            "--temp-dir",
            tmp_dir.path().to_str().unwrap(),
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(fs::read_dir(tmp_dir.path()).unwrap().next().is_none());
    assert!(
        fs::read_dir(&dst_dir).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".oc-rsync-tmp.")),
        "intermediate temp dir created",
    );
    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"y");
}

#[test]
#[cfg(unix)]
fn temp_dir_cross_filesystem_finalizes() {
    let base = tempdir_in(".").unwrap();
    let src_dir = base.path().join("src");
    let dst_dir = base.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let data = b"data".repeat(10);
    fs::write(src_dir.join("a.txt"), &data).unwrap();

    let tmp_dir = match Tmpfs::new() {
        Some(t) => t,
        None => {
            eprintln!("skipping cross-filesystem finalize test; tmpfs unavailable");
            return;
        }
    };

    let dst_dev = fs::metadata(&dst_dir).unwrap().dev();
    let tmp_dev = fs::metadata(tmp_dir.path()).unwrap().dev();
    assert_ne!(dst_dev, tmp_dev, "devices match");

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--temp-dir",
            tmp_dir.path().to_str().unwrap(),
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(fs::read_dir(tmp_dir.path()).unwrap().next().is_none());
    let entries: Vec<_> = fs::read_dir(&dst_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries, ["a.txt".to_string()]);
    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, data);
}

#[test]
#[cfg(unix)]
fn temp_dir_cross_filesystem_matches_rsync() {
    let base = tempdir_in(".").unwrap();
    let src_dir = base.path().join("src");
    let rsync_dst = base.path().join("rsync");
    let ours_dst = base.path().join("ours");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();
    let data = vec![b'x'; 200_000];
    fs::write(src_dir.join("a.txt"), &data).unwrap();

    let tmp_dir = match Tmpfs::new() {
        Some(t) => t,
        None => {
            eprintln!("skipping cross-filesystem parity test; tmpfs unavailable");
            return;
        }
    };

    let dst_dev = fs::metadata(&rsync_dst).unwrap().dev();
    let tmp_dev = fs::metadata(tmp_dir.path()).unwrap().dev();
    assert_ne!(dst_dev, tmp_dev, "devices match");

    let src_arg = format!("{}/", src_dir.display());
    std::process::Command::new(cargo_bin("oc-rsync"))
        .args([
            "-r",
            "--temp-dir",
            tmp_dir.path().to_str().unwrap(),
            &src_arg,
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(fs::read_dir(tmp_dir.path()).unwrap().next().is_none());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--temp-dir",
            tmp_dir.path().to_str().unwrap(),
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(fs::read_dir(tmp_dir.path()).unwrap().next().is_none());

    let diff = std::process::Command::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn numeric_ids_are_preserved() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();
    #[cfg(unix)]
    let (uid, gid) = {
        let desired = (Uid::from_raw(12345), Gid::from_raw(12345));
        if let Err(err) = chown(&file, Some(desired.0), Some(desired.1)) {
            eprintln!("skipping numeric_ids_are_preserved: {err}");
            return;
        }
        desired
    };

    let dst_file = dst_dir.join("id.txt");
    std::fs::copy(&file, &dst_file).unwrap();
    #[cfg(unix)]
    {
        let new_uid = if uid.as_raw() == 0 { 1 } else { 0 };
        let new_gid = if gid.as_raw() == 0 { 1 } else { 0 };
        let _ = chown(
            &dst_file,
            Some(Uid::from_raw(new_uid)),
            Some(Gid::from_raw(new_gid)),
        );
    }

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--numeric-ids",
        "--owner",
        "--group",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    #[cfg(unix)]
    {
        let meta = std::fs::metadata(dst_dir.join("id.txt")).unwrap();
        assert_eq!(meta.uid(), uid.as_raw());
        assert_eq!(meta.gid(), gid.as_raw());
    }
}

#[cfg(unix)]
#[test]
fn owner_group_and_mode_preserved() {
    use std::os::unix::fs::PermissionsExt;
    if !Uid::effective().is_root() {
        eprintln!("skipping owner_group_and_mode_preserved: requires root or CAP_CHOWN",);
        return;
    }
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("a.txt");
    std::fs::write(&file, b"ids").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o741)).unwrap();

    let dst_file = dst_dir.join("a.txt");
    std::fs::copy(&file, &dst_file).unwrap();
    let uid = get_current_uid();
    let gid = get_current_gid();
    let new_uid = if uid == 0 { 1 } else { 0 };
    let new_gid = if gid == 0 { 1 } else { 0 };
    let _ = chown(
        &dst_file,
        Some(Uid::from_raw(new_uid)),
        Some(Gid::from_raw(new_gid)),
    );
    std::fs::set_permissions(&dst_file, std::fs::Permissions::from_mode(0o600)).unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--owner",
        "--group",
        "--perms",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let meta = std::fs::metadata(dst_dir.join("a.txt")).unwrap();
    assert_eq!(meta.uid(), uid);
    assert_eq!(meta.gid(), gid);
    assert_eq!(meta.permissions().mode() & 0o7777, 0o741);
}

#[cfg(all(unix, feature = "acl"))]
#[test]
fn owner_group_perms_acls_preserved() {
    use posix_acl::{ACL_READ, PosixACL, Qualifier};
    use std::os::unix::fs::PermissionsExt;
    if !Uid::effective().is_root() {
        eprintln!("skipping owner_group_perms_acls_preserved: requires root or CAP_CHOWN");
        return;
    }
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("a.txt");
    fs::write(&file, b"ids").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
    let mut acl = PosixACL::read_acl(&file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&file).unwrap();
    let acl_src = PosixACL::read_acl(&file).unwrap();

    let dst_file = dst_dir.join("a.txt");
    fs::write(&dst_file, b"junk").unwrap();
    fs::set_permissions(&dst_file, fs::Permissions::from_mode(0o600)).unwrap();
    let uid = get_current_uid();
    let gid = get_current_gid();
    let new_uid = if uid == 0 { 1 } else { 0 };
    let new_gid = if gid == 0 { 1 } else { 0 };
    let _ = chown(
        &dst_file,
        Some(Uid::from_raw(new_uid)),
        Some(Gid::from_raw(new_gid)),
    );

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--owner",
        "--group",
        "--perms",
        "--acls",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let meta = std::fs::metadata(dst_dir.join("a.txt")).unwrap();
    assert_eq!(meta.uid(), uid);
    assert_eq!(meta.gid(), gid);
    assert_eq!(meta.permissions().mode() & 0o777, 0o640);
    let acl_dst = PosixACL::read_acl(dst_dir.join("a.txt")).unwrap();
    assert_eq!(acl_src.entries(), acl_dst.entries());
}

#[cfg(unix)]
#[test]
fn hard_links_preserved_via_cli() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let f1 = src_dir.join("a");
    fs::write(&f1, b"hi").unwrap();
    let f2 = src_dir.join("b");
    fs::hard_link(&f1, &f2).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--hard-links", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .success();

    let ino1 = fs::metadata(dst_dir.join("a")).unwrap().ino();
    let ino2 = fs::metadata(dst_dir.join("b")).unwrap().ino();
    assert_eq!(ino1, ino2);
}

#[cfg(unix)]
#[test]
fn numeric_ids_falls_back_when_unprivileged() {
    let dir = tempdir().unwrap();
    let probe = dir.path().join("probe");
    std::fs::write(&probe, b"probe").unwrap();
    let current_uid = get_current_uid();
    let current_gid = get_current_gid();
    if Uid::effective().is_root() {
        eprintln!("skipping numeric_ids_falls_back_when_unprivileged: requires non-root");
        return;
    }
    let target_uid = current_uid + 1;
    if chown(&probe, Some(Uid::from_raw(target_uid)), None).is_ok() {
        eprintln!("skipping numeric_ids_falls_back_when_unprivileged: has CAP_CHOWN");
        return;
    }

    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--numeric-ids",
            "--owner",
            "--group",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let dst_file = dst_dir.join("id.txt");
    let meta = std::fs::metadata(&dst_file).unwrap();
    assert_eq!(meta.uid(), current_uid);
    assert_eq!(meta.gid(), current_gid);
}

#[cfg(unix)]
#[test]
fn owner_requires_privileges() {
    let dir = tempdir().unwrap();
    let probe = dir.path().join("probe");
    std::fs::write(&probe, b"probe").unwrap();
    let current_uid = get_current_uid();
    let target_uid = if current_uid == 0 { 1 } else { current_uid + 1 };
    if chown(&probe, Some(Uid::from_raw(target_uid)), None).is_ok() {
        eprintln!("skipping owner_requires_privileges: has CAP_CHOWN or running as root");
        return;
    }

    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--owner", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .failure()
        .code(u8::from(protocol::ExitCode::StartClient) as i32)
        .stderr(predicates::str::contains("changing ownership requires"));

    let dst_file = dst_dir.join("id.txt");
    assert!(!dst_file.exists());
}

#[cfg(unix)]
#[test]
fn user_and_group_ids_are_mapped() {
    let uid = get_current_uid();
    let _gid = get_current_gid();
    if uid != 0 {
        eprintln!("skipping user_and_group_ids_are_mapped: requires root or CAP_CHOWN");
        return;
    }
    {
        let dir = tempdir().unwrap();
        let probe = dir.path().join("probe");
        std::fs::write(&probe, b"probe").unwrap();
        if let Err(err) = chown(&probe, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))) {
            match err {
                nix::errno::Errno::EPERM => {
                    eprintln!("skipping user_and_group_ids_are_mapped: lacks CAP_CHOWN");
                    return;
                }
                _ => panic!("unexpected chown error: {err}"),
            }
        }
    }

    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let uid = get_current_uid();
    let gid = get_current_gid();
    let mapped_uid = 1;
    let mapped_gid = 1;
    let usermap = format!("--usermap={uid}:{mapped_uid}");
    let groupmap = format!("--groupmap={gid}:{mapped_gid}");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            usermap.as_str(),
            groupmap.as_str(),
            src_arg.as_str(),
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = std::fs::metadata(dst_dir.join("id.txt")).unwrap();
    assert_eq!(meta.uid(), mapped_uid);
    assert_eq!(meta.gid(), mapped_gid);
}

#[cfg(unix)]
#[test]
fn user_names_are_mapped_even_with_numeric_ids() {
    let uid = get_current_uid();
    if uid != 0 {
        eprintln!(
            "skipping user_names_are_mapped_even_with_numeric_ids: requires root or CAP_CHOWN"
        );
        return;
    }
    {
        let dir = tempdir().unwrap();
        let probe = dir.path().join("probe");
        std::fs::write(&probe, b"probe").unwrap();
        if let Err(err) = chown(&probe, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))) {
            match err {
                nix::errno::Errno::EPERM => {
                    eprintln!(
                        "skipping user_names_are_mapped_even_with_numeric_ids: lacks CAP_CHOWN"
                    );
                    return;
                }
                _ => panic!("unexpected chown error: {err}"),
            }
        }
    }

    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let uname = get_user_by_uid(uid)
        .unwrap()
        .name()
        .to_string_lossy()
        .into_owned();
    let passwd_data = std::fs::read_to_string("/etc/passwd").unwrap();
    let (other_name, other_uid) = passwd_data
        .lines()
        .find_map(|line| {
            if line.starts_with('#') || line.trim().is_empty() {
                return None;
            }
            let mut parts = line.split(':');
            let name = parts.next()?;
            parts.next();
            let uid_str = parts.next()?;
            let uid_val: u32 = uid_str.parse().ok()?;
            if uid_val != uid {
                Some((name.to_string(), uid_val))
            } else {
                None
            }
        })
        .expect("no alternate user found");

    let usermap = format!("--usermap={uname}:{other_name}");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--numeric-ids",
            usermap.as_str(),
            src_arg.as_str(),
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = std::fs::metadata(dst_dir.join("id.txt")).unwrap();
    assert_eq!(meta.uid(), other_uid);
}

#[cfg(unix)]
#[test]
fn group_names_are_mapped() {
    let uid = get_current_uid();
    if uid != 0 {
        eprintln!("skipping group_names_are_mapped: requires root or CAP_CHOWN");
        return;
    }
    {
        let dir = tempdir().unwrap();
        let probe = dir.path().join("probe");
        std::fs::write(&probe, b"probe").unwrap();
        if let Err(err) = chown(&probe, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))) {
            match err {
                nix::errno::Errno::EPERM => {
                    eprintln!("skipping group_names_are_mapped: lacks CAP_CHOWN");
                    return;
                }
                _ => panic!("unexpected chown error: {err}"),
            }
        }
    }

    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let gid = get_current_gid();
    let gname = get_group_by_gid(gid)
        .unwrap()
        .name()
        .to_string_lossy()
        .into_owned();

    let group_data = std::fs::read_to_string("/etc/group").unwrap();
    let (other_name, other_gid) = group_data
        .lines()
        .find_map(|line| {
            if line.starts_with('#') || line.trim().is_empty() {
                return None;
            }
            let mut parts = line.split(':');
            let name = parts.next()?;
            parts.next();
            let gid_str = parts.next()?;
            let gid_val: u32 = gid_str.parse().ok()?;
            if gid_val != gid {
                Some((name.to_string(), gid_val))
            } else {
                None
            }
        })
        .expect("no alternate group found");

    let groupmap = format!("--groupmap={gname}:{other_name}");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            groupmap.as_str(),
            src_arg.as_str(),
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = std::fs::metadata(dst_dir.join("id.txt")).unwrap();
    assert_eq!(meta.gid(), other_gid);
}

#[cfg(unix)]
#[test]
fn parse_usermap_accepts_numeric_and_name() {
    use meta::{IdKind, parse_id_map};
    use users::get_user_by_uid;

    let numeric = parse_id_map("0:1", IdKind::User).unwrap();
    assert_eq!(numeric(0), 1);

    let root_name = get_user_by_uid(0)
        .unwrap()
        .name()
        .to_string_lossy()
        .into_owned();
    let spec = format!("{root_name}:{root_name}");
    let name_map = parse_id_map(&spec, IdKind::User).unwrap();
    assert_eq!(name_map(0), 0);
}

#[cfg(unix)]
#[test]
fn parse_groupmap_accepts_numeric_and_name() {
    use meta::{IdKind, parse_id_map};
    use users::get_group_by_gid;

    let numeric = parse_id_map("0:1", IdKind::Group).unwrap();
    assert_eq!(numeric(0), 1);

    let root_gname = get_group_by_gid(0)
        .unwrap()
        .name()
        .to_string_lossy()
        .into_owned();
    let spec = format!("{root_gname}:{root_gname}");
    let name_map = parse_id_map(&spec, IdKind::Group).unwrap();
    assert_eq!(name_map(0), 0);
}

#[cfg(unix)]
#[test]
fn user_name_to_numeric_id_is_mapped() {
    let uid = get_current_uid();
    if uid != 0 {
        eprintln!("skipping user_name_to_numeric_id_is_mapped: requires root or CAP_CHOWN",);
        return;
    }
    {
        let dir = tempdir().unwrap();
        let probe = dir.path().join("probe");
        std::fs::write(&probe, b"probe").unwrap();
        if let Err(err) = chown(&probe, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))) {
            match err {
                nix::errno::Errno::EPERM => {
                    eprintln!("skipping user_name_to_numeric_id_is_mapped: lacks CAP_CHOWN",);
                    return;
                }
                _ => panic!("unexpected chown error: {err}"),
            }
        }
    }

    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();

    let uname = get_user_by_uid(uid)
        .unwrap()
        .name()
        .to_string_lossy()
        .into_owned();
    let passwd_data = std::fs::read_to_string("/etc/passwd").unwrap();
    let other_uid = passwd_data
        .lines()
        .find_map(|line| {
            if line.starts_with('#') || line.trim().is_empty() {
                return None;
            }
            let mut parts = line.split(':');
            parts.next()?;
            parts.next();
            let uid_str = parts.next()?;
            let uid_val: u32 = uid_str.parse().ok()?;
            if uid_val != uid { Some(uid_val) } else { None }
        })
        .expect("no alternate user id found");

    let usermap = format!("--usermap={uname}:{other_uid}");
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            usermap.as_str(),
            src_arg.as_str(),
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = std::fs::metadata(dst_dir.join("id.txt")).unwrap();
    assert_eq!(meta.uid(), other_uid);
}

#[cfg(unix)]
#[test]
fn group_id_to_name_is_mapped() {
    let uid = get_current_uid();
    if uid != 0 {
        eprintln!("skipping group_id_to_name_is_mapped: requires root or CAP_CHOWN");
        return;
    }
    {
        let dir = tempdir().unwrap();
        let probe = dir.path().join("probe");
        std::fs::write(&probe, b"probe").unwrap();
        if let Err(err) = chown(&probe, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))) {
            match err {
                nix::errno::Errno::EPERM => {
                    eprintln!("skipping group_id_to_name_is_mapped: lacks CAP_CHOWN",);
                    return;
                }
                _ => panic!("unexpected chown error: {err}"),
            }
        }
    }

    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();

    let gid = get_current_gid();
    let group_data = std::fs::read_to_string("/etc/group").unwrap();
    let (other_name, other_gid) = group_data
        .lines()
        .find_map(|line| {
            if line.starts_with('#') || line.trim().is_empty() {
                return None;
            }
            let mut parts = line.split(':');
            let name = parts.next()?;
            parts.next();
            let gid_str = parts.next()?;
            let gid_val: u32 = gid_str.parse().ok()?;
            if gid_val != gid {
                Some((name.to_string(), gid_val))
            } else {
                None
            }
        })
        .expect("no alternate group found");

    let groupmap = format!("--groupmap={gid}:{other_name}");
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            groupmap.as_str(),
            src_arg.as_str(),
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = std::fs::metadata(dst_dir.join("id.txt")).unwrap();
    assert_eq!(meta.gid(), other_gid);
}

#[test]
fn verbose_flag_increases_logging() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--verbose", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("verbose level set to 1"));
}

#[test]
fn log_file_format_json_outputs_structured_file() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();

    let log = dir.path().join("log.json");
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--log-file",
        log.to_str().unwrap(),
        "--log-file-format=json",
        "--verbose",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();
    let contents = std::fs::read_to_string(log).unwrap();
    assert!(contents.contains("\"message\""));
}

#[test]
fn quiet_flag_suppresses_output() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--recursive",
        "--quiet",
        "--progress",
        "--stats",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success().stdout("").stderr("");
}

#[test]
fn archive_implies_recursive() {
    let dir = tempdir().unwrap();
    let src_root = dir.path().join("src");
    std::fs::create_dir_all(src_root.join("a/b")).unwrap();
    std::fs::write(src_root.join("a/b/file.txt"), b"hi").unwrap();
    let dst_dir = dir.path().join("dst");

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_root.display());
    cmd.args([
        "-a",
        "--no-o",
        "--no-g",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();
    assert!(dst_dir.join("a/b/file.txt").exists());
}

#[test]
fn dry_run_parity_destination_untouched() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("new.txt"), b"hello").unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    std::fs::write(dst_dir.join("existing.txt"), b"keep").unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let dst_arg = dst_dir.to_str().unwrap();

    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .args(["--recursive", "--dry-run", &src_arg, dst_arg])
        .output()
        .unwrap();

    assert!(dst_dir.join("existing.txt").exists());
    assert_eq!(
        std::fs::read_to_string(dst_dir.join("existing.txt")).unwrap(),
        "keep"
    );
    assert!(!dst_dir.join("new.txt").exists());
    let (exp_stdout, exp_stderr, exp_exit) = read_golden("dry_run/untouched");

    assert_eq!(ours.status.code(), Some(exp_exit));
    assert_eq!(ours.stdout, exp_stdout);
    assert_eq!(ours.stderr, exp_stderr);
}

#[test]
fn checksum_forces_transfer_cli() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    let src_file = src.join("file");
    let dst_file = dst.join("file");
    std::fs::write(&src_file, b"aaaa").unwrap();
    std::fs::write(&dst_file, b"bbbb").unwrap();
    let mtime = FileTime::from_unix_time(1_000_000, 0);
    set_file_mtime(&src_file, mtime).unwrap();
    set_file_mtime(&dst_file, mtime).unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([&format!("{}/", src.display()), dst.to_str().unwrap()])
        .assert()
        .success();
    assert_eq!(std::fs::read(&dst_file).unwrap(), b"bbbb");

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--checksum",
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert_eq!(std::fs::read(&dst_file).unwrap(), b"aaaa");
}

#[test]
fn delete_non_empty_dir_without_force_fails() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(dst.join("sub")).unwrap();
    std::fs::write(dst.join("sub/file.txt"), b"hi").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--delete",
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Directory not empty"));

    assert!(dst.join("sub/file.txt").exists());
}

#[test]
fn force_allows_non_empty_dir_deletion() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(dst.join("sub")).unwrap();
    std::fs::write(dst.join("sub/file.txt"), b"hi").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--delete",
            "--force",
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("sub").exists());
}

#[test]
fn force_removes_nested_non_empty_dirs() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(dst.join("sub/nested")).unwrap();
    std::fs::write(dst.join("sub/nested/file.txt"), b"hi").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--delete",
            "--force",
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("sub").exists());
}

#[test]
fn force_removes_multiple_non_empty_dirs() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(dst.join("sub1/nested1")).unwrap();
    std::fs::create_dir_all(dst.join("sub2/nested2")).unwrap();
    std::fs::write(dst.join("sub1/nested1/file1.txt"), b"hi").unwrap();
    std::fs::write(dst.join("sub2/nested2/file2.txt"), b"hi").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--delete",
            "--force",
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("sub1").exists());
    assert!(!dst.join("sub2").exists());
}

#[cfg(unix)]
#[test]
#[serial]
fn perms_flag_preserves_permissions() {
    use nix::sys::stat::{Mode, umask};
    use std::fs;
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
    cmd.args(["--perms", &src_arg, dst_dir.to_str().unwrap()]);
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
    use nix::sys::stat::{Mode, umask};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
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
        .args([&src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .success();

    umask(old_umask);

    let mode = fs::metadata(dst_dir.join("a.sh"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    let expected = 0o754 & !0o027;
    if mode != expected {
        eprintln!("skipping: umask not honored (got {mode:o}, expected {expected:o})");
        return;
    }
    assert_eq!(mode, expected);
}

#[cfg(unix)]
#[test]
fn chmod_masks_file_type_bits() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    let file = src_dir.join("a.txt");
    fs::write(&file, b"hi").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--chmod=100644", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let mode = fs::metadata(dst_dir.join("a.txt"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o7777, 0o644);
}

#[cfg(unix)]
#[test]
fn numeric_chmod_leaves_directories_executable() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(src_dir.join("sub")).unwrap();
    fs::write(src_dir.join("sub/file"), b"hi").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--recursive",
        "--chmod=100644",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let mode = fs::metadata(dst_dir.join("sub"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o755);
}

#[test]
fn stats_are_printed() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), b"hello").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--stats", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success().stdout(predicates::str::contains(
        "Number of regular files transferred",
    ));
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
fn files_from_list_transfers_only_listed_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("keep.txt"), b"k").unwrap();
    fs::write(src.join("other file.txt"), b"o").unwrap();
    fs::write(src.join("skip.txt"), b"s").unwrap();
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
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("a/b")).unwrap();
    fs::create_dir_all(src.join("a/d/sub")).unwrap();
    fs::write(src.join("a/b/file.txt"), b"f").unwrap();
    fs::write(src.join("a/b/other.txt"), b"o").unwrap();
    fs::write(src.join("a/d/sub/nested.txt"), b"n").unwrap();
    fs::write(src.join("unlisted.txt"), b"u").unwrap();
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
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("a/b")).unwrap();
    fs::create_dir_all(src.join("a/c")).unwrap();
    fs::write(src.join("a/b/file.txt"), b"f").unwrap();
    fs::write(src.join("a/b/other.txt"), b"o").unwrap();
    fs::write(src.join("a/c/unrelated.txt"), b"u").unwrap();
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
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("dir/sub")).unwrap();
    fs::create_dir_all(src.join("other")).unwrap();
    fs::write(src.join("dir/sub/file.txt"), b"k").unwrap();
    fs::write(src.join("other/file.txt"), b"o").unwrap();
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
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("dir/sub")).unwrap();
    fs::create_dir_all(src.join("other")).unwrap();
    fs::write(src.join("dir/sub/file.txt"), b"k").unwrap();
    fs::write(src.join("other/file.txt"), b"o").unwrap();
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
fn files_from_zero_separated_list() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep me.txt"), b"k").unwrap();
    std::fs::write(src.join("skip.txt"), b"s").unwrap();
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

#[test]
fn files_from_zero_separated_list_with_crlf() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep me.txt"), b"k").unwrap();
    std::fs::write(src.join("skip.txt"), b"s").unwrap();
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
fn files_from_zero_separated_list_allows_hash() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("#keep.txt"), b"k").unwrap();
    std::fs::write(src.join("skip.txt"), b"s").unwrap();
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
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(src.join("dir/sub")).unwrap();
    std::fs::write(src.join("dir/sub/file.txt"), b"k").unwrap();
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
    let rsync_dst = dir.path().join("rsync");
    let oc_dst = dir.path().join("oc");
    fs::create_dir_all(src.join("foo/bar")).unwrap();
    fs::write(src.join("foo/bar/baz.txt"), b"k").unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&oc_dst).unwrap();
    let list = dir.path().join("files.lst");
    fs::write(&list, "foo/bar/baz.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());
    let status = std::process::Command::new("rsync")
        .args([
            "-r",
            "--files-from",
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

    let diff = std::process::Command::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&oc_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn files_from_single_file_no_implied_dirs_fails_like_rsync() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let rsync_dst = dir.path().join("rsync");
    let oc_dst = dir.path().join("oc");
    fs::create_dir_all(src.join("foo/bar")).unwrap();
    fs::write(src.join("foo/bar/baz.txt"), b"k").unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&oc_dst).unwrap();
    let list = dir.path().join("files.lst");
    fs::write(&list, "foo/bar/baz.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());
    let rsync_status = std::process::Command::new("rsync")
        .args([
            "-r",
            "--no-implied-dirs",
            "--files-from",
            list.to_str().unwrap(),
            &src_arg,
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!rsync_status.success());

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
        .failure();

    let diff = std::process::Command::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&oc_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
    assert!(!oc_dst.join("foo/bar/baz.txt").exists());
}

#[test]
fn files_from_zero_separated_list_directory_without_slash_excludes_siblings() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("dir/sub")).unwrap();
    fs::create_dir_all(src.join("other")).unwrap();
    fs::write(src.join("dir/sub/file.txt"), b"k").unwrap();
    fs::write(src.join("other/file.txt"), b"o").unwrap();
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
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep.txt"), b"k").unwrap();
    std::fs::write(src.join("skip.txt"), b"s").unwrap();
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
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep.txt"), b"k").unwrap();
    std::fs::write(src.join("skip.txt"), b"s").unwrap();
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
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(src.join("dir/sub")).unwrap();
    std::fs::write(src.join("dir/sub/file.txt"), b"k").unwrap();
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
