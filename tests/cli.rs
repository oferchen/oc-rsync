// tests/cli.rs

use assert_cmd::prelude::*;
use assert_cmd::Command;
use filetime::{set_file_mtime, FileTime};
#[cfg(unix)]
use nix::unistd::{chown, mkfifo, Gid, Uid};
use predicates::prelude::PredicateBooleanExt;
use std::fs;
use std::io::{Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
#[cfg(unix)]
use users::{get_current_gid, get_current_uid};

#[test]
fn prints_version() {
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.arg("--version");
    let expected = format!("oc-rsync {}\n", env!("CARGO_PKG_VERSION"));
    cmd.assert().success().stdout(expected).stderr("");
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
    let path_line = stderr.lines().next().unwrap();
    assert_eq!(path_line, dst_dir.join("a.txt").display().to_string());
    let progress_line = stderr.split('\r').next_back().unwrap().trim_end();
    assert_eq!(progress_line, format!("{:>15} {:>3}%", "2,048", 100));
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
    let path_line = stderr.lines().next().unwrap();
    assert_eq!(path_line, dst_dir.join("a.txt").display().to_string());
    let progress_line = stderr.split('\r').next_back().unwrap().trim_end();
    assert_eq!(progress_line, format!("{:>15} {:>3}%", "2.00KiB", 100));
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
fn resumes_from_partial_dir_with_subdirs() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    let partial_dir = dir.path().join("partial");
    std::fs::create_dir_all(src_dir.join("sub")).unwrap();
    std::fs::write(src_dir.join("sub/a.txt"), b"hello").unwrap();
    std::fs::create_dir_all(partial_dir.join("sub")).unwrap();
    std::fs::write(partial_dir.join("sub/a.txt"), b"he").unwrap();

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
    assert!(!partial_dir.join("sub/a.txt").exists());
    assert!(!partial_dir.join("sub").exists());
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

    let tmp_file = tmp_dir.join("a.tmp");
    let mut found = false;
    for _ in 0..50 {
        if tmp_file.exists() {
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

    thread::sleep(Duration::from_millis(500));
    let out = std::fs::read(&dst_file).unwrap();
    assert_eq!(out, b"old");
    assert!(dst_dir.join("a.tmp").exists());

    child.wait().unwrap();
    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out.len(), 50_000);
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
        match chown(&file, Some(desired.0), Some(desired.1)) {
            Ok(_) => desired,
            Err(_) => {
                let meta = std::fs::metadata(&file).unwrap();
                (Uid::from_raw(meta.uid()), Gid::from_raw(meta.gid()))
            }
        }
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
    cmd.args(["--local", "-a", &src_arg, dst_dir.to_str().unwrap()]);
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

#[cfg(unix)]
#[test]
fn perms_flag_preserves_permissions() {
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

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--local", "--perms", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let mode = fs::metadata(dst_dir.join("a.txt"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o7777, 0o741);
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
    fs::set_permissions(&dst_dir, fs::Permissions::from_mode(0o755)).unwrap();

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
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("files transferred"));
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
