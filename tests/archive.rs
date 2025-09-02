// tests/archive.rs
#[cfg(unix)]
use assert_cmd::Command;
#[cfg(unix)]
use filetime::{set_file_mtime, FileTime};
#[cfg(unix)]
use nix::sys::stat::{mknod, Mode, SFlag};
#[cfg(unix)]
use nix::unistd::{chown, mkfifo, Gid, Uid};
#[cfg(unix)]
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::{symlink, PermissionsExt};
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::process::Command as StdCommand;
#[cfg(unix)]
use tempfile::tempdir;

#[cfg(unix)]
fn hash_dir(dir: &Path) -> Vec<u8> {
    let output = StdCommand::new("tar")
        .args(["--numeric-owner", "-cf", "-", "."])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut hasher = Sha256::new();
    hasher.update(&output.stdout);
    hasher.finalize().to_vec()
}

#[cfg(unix)]
#[test]
fn archive_matches_combination_and_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::create_dir(src.join("dir")).unwrap();
    fs::write(src.join("dir/file"), b"hi").unwrap();
    set_file_mtime(src.join("dir/file"), FileTime::from_unix_time(1_234_567, 0)).unwrap();
    fs::set_permissions(src.join("dir/file"), fs::Permissions::from_mode(0o640)).unwrap();
    chown(
        src.join("dir/file").as_path(),
        Some(Uid::from_raw(42)),
        Some(Gid::from_raw(43)),
    )
    .unwrap();
    symlink("dir/file", src.join("link")).unwrap();
    mkfifo(&src.join("fifo"), Mode::from_bits_truncate(0o644)).unwrap();
    mknod(
        &src.join("dev"),
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o644),
        meta::makedev(1, 7),
    )
    .unwrap();

    let dst_archive = tmp.path().join("dst_archive");
    let dst_combo = tmp.path().join("dst_combo");
    let dst_rsync = tmp.path().join("dst_rsync");

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "-a",
            &format!("{}/", src.display()),
            dst_archive.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--links",
            "--perms",
            "--times",
            "--group",
            "--owner",
            "--devices",
            "--specials",
            &format!("{}/", src.display()),
            dst_combo.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(StdCommand::new("rsync")
        .args([
            "-a",
            &format!("{}/", src.display()),
            dst_rsync.to_str().unwrap(),
        ])
        .status()
        .unwrap()
        .success());

    let h_archive = hash_dir(&dst_archive);
    let h_combo = hash_dir(&dst_combo);
    let h_rsync = hash_dir(&dst_rsync);
    assert_eq!(h_archive, h_combo);
    assert_eq!(h_archive, h_rsync);
}
