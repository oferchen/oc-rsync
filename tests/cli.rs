// tests/cli.rs

use assert_cmd::prelude::*;
use assert_cmd::Command;
use engine::SyncOptions;
use filetime::{set_file_mtime, FileTime};
use logging::progress_formatter;
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
use std::process::Command as StdCommand;
use std::thread;
use std::time::Duration;
use tempfile::{tempdir, tempdir_in, TempDir};
#[cfg(unix)]
use users::{get_current_gid, get_current_uid, get_group_by_gid, get_user_by_uid};
#[cfg(all(unix, feature = "xattr"))]
use xattr as _;

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
    use std::process::Command as StdCommand;

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

    StdCommand::new("rsync")
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

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
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

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn include_from_from0_matches_rsync() {
    use std::process::Command as StdCommand;

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

    StdCommand::new("rsync")
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

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
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

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn exclude_from_from0_matches_rsync() {
    use std::process::Command as StdCommand;

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

    StdCommand::new("rsync")
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

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--from0",
            "--exclude-from",
            list.to_str().unwrap(),
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn filter_file_from0_matches_rsync() {
    use std::process::Command as StdCommand;

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("a.txt"), "hi").unwrap();
    fs::write(src.join("b.log"), "no").unwrap();
    fs::write(src.join("c.txt"), "hi").unwrap();

    let filter = tmp.path().join("filters");
    fs::write(&filter, b"+ *.txt\0- *\0").unwrap();

    let src_arg = format!("{}/", src.display());

    StdCommand::new("rsync")
        .args([
            "-r",
            "--from0",
            "--filter",
            &format!("merge {}", filter.display()),
            &src_arg,
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--from0",
            "--filter-file",
            filter.to_str().unwrap(),
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn per_dir_merge_matches_rsync() {
    use std::process::Command as StdCommand;

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("keep.txt"), "hi").unwrap();
    fs::write(src.join("omit.log"), "no").unwrap();
    fs::write(src.join("sub").join("keep2.txt"), "hi").unwrap();
    fs::write(src.join("sub").join("omit2.txt"), "no").unwrap();

    fs::write(src.join(".rsync-filter"), b"- *.log\n").unwrap();
    fs::write(src.join("sub").join(".rsync-filter"), b"- omit2.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());

    StdCommand::new("rsync")
        .args(["-r", "-F", "-F", &src_arg, rsync_dst.to_str().unwrap()])
        .status()
        .unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "-F",
            "-F",
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
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
    let expected = format!(
        "oc-rsync {} (protocol {})\nrsync {}\n{} {}\n",
        env!("CARGO_PKG_VERSION"),
        SUPPORTED_PROTOCOLS[0],
        option_env!("RSYNC_UPSTREAM_VER").unwrap_or("unknown"),
        option_env!("BUILD_REVISION").unwrap_or("unknown"),
        option_env!("OFFICIAL_BUILD").unwrap_or("unofficial"),
    );
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
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--iconv=FOO",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
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
    let local = cv.to_local(remote);
    assert_eq!(local, b"f\xC3\xB8o");
    let roundtrip = cv.to_remote(&local);
    assert_eq!(roundtrip, remote);
}

#[test]
fn client_local_sync() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), b"hello world").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--local", &src_arg, dst_dir.to_str().unwrap()]);
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
    cmd.args([
        "--local",
        "--whole-file",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
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
        .args([
            "--local",
            "--ignore-existing",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
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
        .args(["--local", "--existing", &src_arg, dst_dir.to_str().unwrap()])
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
        .args(["--local", "--existing", &src_arg, dst_dir.to_str().unwrap()])
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
        .args([
            "--local",
            "--size-only",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
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
        .args([
            "--local",
            "--ignore-times",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"new");
}

#[test]
fn local_sync_without_flag_fails() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([&src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().failure();
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
        .args(["--local", "-src/", "dst"])
        .assert()
        .failure();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .current_dir(dir.path())
        .args(["--local", "--", "-src/", "dst"])
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
    cmd.args(["--local", "-R", "src/a/b/", "dst"]);
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
    std::fs::write(src_dir.join("a.txt"), vec![0u8; 2048]).unwrap();
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    let assert = cmd
        .args(["--local", "--progress", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    let mut lines = stderr.lines();
    assert_eq!(lines.next().unwrap(), "sending incremental file list");
    assert_eq!(lines.next().unwrap(), "a.txt");
    let progress_line = lines.next().unwrap().trim_start_matches('\r').trim_end();
    let bytes = progress_formatter(2048, false);
    let expected_prefix = format!("{:>15} {:>3}%", bytes, 100);
    assert!(progress_line.starts_with(&expected_prefix));
}

#[test]
fn progress_parity() {
    let rsync = StdCommand::new("rsync")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok();
    if rsync.is_none() {
        eprintln!("skipping test: rsync not installed");
        return;
    }

    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst_up = dir.path().join("dst_up");
    let dst_ours = dir.path().join("dst_ours");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), b"hello").unwrap();

    let up = StdCommand::new("rsync")
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .args(["-r", "--progress"])
        .arg(format!("{}/", src.display()))
        .arg(&dst_up)
        .output()
        .unwrap();
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .args([
            "--local",
            "--progress",
            format!("{}/", src.display()).as_str(),
            dst_ours.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let norm = |s: &[u8]| {
        let txt = String::from_utf8_lossy(s).replace('\r', "\n");
        txt.lines()
            .rev()
            .find(|l| l.contains('%'))
            .and_then(|l| l.split(" (xfr").next())
            .unwrap()
            .to_string()
    };
    let up_line = norm(&up.stdout);
    let our_line = norm(&ours.stderr);

    assert_eq!(our_line, up_line);
    insta::assert_snapshot!("progress_parity", our_line);
}

#[test]
fn stats_parity() {
    let rsync = StdCommand::new("rsync")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok();
    if rsync.is_none() {
        eprintln!("skipping test: rsync not installed");
        return;
    }

    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst_up = dir.path().join("dst_up");
    let dst_ours = dir.path().join("dst_ours");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), b"hello").unwrap();

    let up = StdCommand::new("rsync")
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .args(["-r", "--stats"])
        .arg(format!("{}/", src.display()))
        .arg(&dst_up)
        .output()
        .unwrap();
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .args([
            "--local",
            "--stats",
            format!("{}/", src.display()).as_str(),
            dst_ours.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let up_stdout = String::from_utf8_lossy(&up.stdout);
    let mut up_stats: Vec<&str> = up_stdout
        .lines()
        .filter(|l| {
            l.starts_with("Number of regular files transferred")
                || l.starts_with("Number of deleted files")
                || l.starts_with("Total transferred file size")
        })
        .collect();
    up_stats.sort_unstable();

    let our_stdout = String::from_utf8_lossy(&ours.stdout);
    let mut our_stats: Vec<&str> = our_stdout.lines().collect();
    our_stats.sort_unstable();

    assert_eq!(our_stats, up_stats);
    insta::assert_snapshot!("stats_parity", our_stats.join("\n"));
}

#[test]
fn progress_flag_human_readable() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();

    std::fs::write(src_dir.join("a.txt"), vec![0u8; 2 * 1024]).unwrap();
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    let assert = cmd
        .args([
            "--local",
            "--progress",
            "--human-readable",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    let mut lines = stderr.lines();
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
        "--local",
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
        "--local",
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
    std::fs::write(dst_dir.join("a.partial"), b"he").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--local", "--partial", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"hello");
    assert!(!dst_dir.join("a.partial").exists());
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
        "--local",
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
            "--local",
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
            "--local",
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
        .args([
            "--local",
            "--bwlimit",
            "20000",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();

    let tmp_file = dst_dir.join("a.tmp");
    let mut found = false;
    for _ in 0..50 {
        if tmp_file.exists() {
            let out = std::fs::read(&dst_file).unwrap();
            assert_eq!(out, b"old");
            found = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        found,
        "temp file not created in destination during transfer"
    );

    child.wait().unwrap();
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
            "--local",
            "--temp-dir",
            tmp_dir.path().to_str().unwrap(),
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
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
            "--local",
            "--delay-updates",
            "--temp-dir",
            tmp_dir.path().to_str().unwrap(),
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
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
            "--local",
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
    std::process::Command::new("rsync")
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
            "--local",
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
        "--local",
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
        "--local",
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
    use posix_acl::{PosixACL, Qualifier, ACL_READ};
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
        "--local",
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
        .args([
            "--local",
            "--hard-links",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
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
            "--local",
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
        .args(["--local", "--owner", &src_arg, dst_dir.to_str().unwrap()])
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
            "--local",
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
            "--local",
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
            "--local",
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
    use meta::{parse_id_map, IdKind};
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
    use meta::{parse_id_map, IdKind};
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
            if uid_val != uid {
                Some(uid_val)
            } else {
                None
            }
        })
        .expect("no alternate user id found");

    let usermap = format!("--usermap={uname}:{other_uid}");
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
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
            "--local",
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
    cmd.args(["--local", "--verbose", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("verbose level set to 1"));
}

#[test]
fn log_format_json_outputs_structured_logs() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--local",
        "--log-format=json",
        "--verbose",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success().stdout(predicates::str::contains(
        "\"message\":\"verbose level set to 1\"",
    ));
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
        "--local",
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
        "--local",
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
        "--local",
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
fn dry_run_does_not_modify_destination() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("file.txt"), b"hello").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--local", "--dry-run", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();
    assert!(!dst_dir.join("file.txt").exists());
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
        .args([
            "--local",
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert_eq!(std::fs::read(&dst_file).unwrap(), b"bbbb");

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
    use nix::sys::stat::{umask, Mode};
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
    use nix::sys::stat::{umask, Mode};
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
    cmd.args([
        "--local",
        "--chmod=100644",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
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
        "--local",
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
    cmd.args(["--local", "--stats", &src_arg, dst_dir.to_str().unwrap()]);
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
        "--local",
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
    cmd.args(["--local", &src_arg, dst_dir.to_str().unwrap()]);
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
            "--local",
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
        .args(["--local", "--links", &src_arg, dst.to_str().unwrap()])
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
        .args(["--local", "--links", &src_arg, dst.to_str().unwrap()])
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
        .args(["--local", "--links", &src_arg, dst.to_str().unwrap()])
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
        .args(["--local", "--links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();
    let meta = std::fs::symlink_metadata(dst.join("link")).unwrap();
    assert!(meta.file_type().is_symlink());
    std::fs::remove_file(dst.join("link")).unwrap();
    std::fs::write(dst.join("link"), b"old").unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
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
        .args(["--local", "--links", &src_arg, dst.to_str().unwrap()])
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
            "--local",
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
        .args([
            "--local",
            "--keep-dirlinks",
            &src_arg,
            dst.to_str().unwrap(),
        ])
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
        .args(["--local", "--copy-links", &src_arg, dst.to_str().unwrap()])
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
        .args(["--local", "--copy-links", &src_arg, dst.to_str().unwrap()])
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
        .args([
            "--local",
            "--links",
            "--safe-links",
            &src_arg,
            dst.to_str().unwrap(),
        ])
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
        .args([
            "--local",
            "--links",
            "--safe-links",
            &src_arg,
            dst.to_str().unwrap(),
        ])
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
        .args([
            "--local",
            "--links",
            "--safe-links",
            &src_arg,
            dst.to_str().unwrap(),
        ])
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
        .args(["--local", "--perms", &src_arg, dst.to_str().unwrap()])
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
        .args(["--local", "--times", &src_arg, dst.to_str().unwrap()])
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
    f.seek(SeekFrom::Start(1 << 20)).unwrap();
    f.write_all(b"end").unwrap();
    f.set_len(1 << 21).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--sparse", &src_arg, dst.to_str().unwrap()])
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
    f.write_all(&vec![0u8; 1 << 20]).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--sparse", &src_arg, dst.to_str().unwrap()])
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
        .args(["--local", "--specials", &src_arg, dst.to_str().unwrap()])
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
            "--local",
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

    let home = tempdir().unwrap();
    fs::write(home.path().join(".cvsignore"), "home_ignored\n").unwrap();

    let src_arg = format!("{}/", src.display());
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.env("CVSIGNORE", "env_ignored");
    cmd.env("HOME", home.path());
    cmd.args([
        "--local",
        "--recursive",
        "--cvs-exclude",
        &src_arg,
        dst.to_str().unwrap(),
    ]);
    cmd.assert().success();

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("core").exists());
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
            "--local",
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
