// tests/misc_tmpfs.rs
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

#[cfg(unix)]
impl Drop for Tmpfs {
    fn drop(&mut self) {
        let _ = std::process::Command::new("umount")
            .arg(self.0.path())
            .status();
    }
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
        "temp file not created in destination during transfer",
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
