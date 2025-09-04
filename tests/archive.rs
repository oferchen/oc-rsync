// tests/archive.rs
#[cfg(unix)]
use assert_cmd::Command;
#[cfg(unix)]
use filetime::{set_file_mtime, FileTime};
#[cfg(unix)]
use nix::unistd::{chown, mkfifo, Gid, Uid};
#[cfg(unix)]
use oc_rsync::meta::{makedev, mknod, Mode, SFlag};
#[cfg(unix)]
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::{symlink, FileTypeExt, MetadataExt, PermissionsExt};
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
#[ignore = "--no-links not yet supported"]
fn archive_matches_combination_and_rsync() {
    if !Uid::effective().is_root() {
        println!("skipping: requires root privileges");
        return;
    }

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::create_dir(src.join("dir")).unwrap();
    fs::write(src.join("dir/file"), b"hi").unwrap();
    set_file_mtime(src.join("dir/file"), FileTime::from_unix_time(1_234_567, 0)).unwrap();
    fs::set_permissions(src.join("dir/file"), fs::Permissions::from_mode(0o666)).unwrap();
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
        makedev(1, 7),
    )
    .unwrap();

    let dst_archive = tmp.path().join("dst_archive");
    let dst_combo = tmp.path().join("dst_combo");
    let dst_rsync = tmp.path().join("dst_rsync");

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-a",
            &format!("{}/", src.display()),
            dst_archive.to_str().unwrap(),
        ])
        .assert()
        .success();

    let dst_file = dst_archive.join("dir/file");
    assert!(dst_file.exists());
    let meta = fs::symlink_metadata(&dst_file).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o666);
    assert_eq!(
        FileTime::from_last_modification_time(&meta).unix_seconds(),
        1_234_567
    );
    assert_eq!(meta.uid(), 42);
    assert_eq!(meta.gid(), 43);
    let link_target = fs::read_link(dst_archive.join("link")).unwrap();
    assert_eq!(link_target, Path::new("dir/file"));
    let fifo_meta = fs::symlink_metadata(dst_archive.join("fifo")).unwrap();
    assert!(fifo_meta.file_type().is_fifo());
    let dev_meta = fs::symlink_metadata(dst_archive.join("dev")).unwrap();
    assert!(dev_meta.file_type().is_char_device());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
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

#[cfg(unix)]
#[test]
#[ignore = "--no-links not yet supported"]
fn archive_respects_no_options() {
    if !Uid::effective().is_root() {
        eprintln!("skipping: requires root privileges");
        return;
    }

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::create_dir(src.join("dir")).unwrap();
    fs::write(src.join("dir/file"), b"hi").unwrap();
    set_file_mtime(src.join("dir/file"), FileTime::from_unix_time(1_234_567, 0)).unwrap();
    fs::set_permissions(src.join("dir/file"), fs::Permissions::from_mode(0o666)).unwrap();
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

    let src_arg = format!("{}/", src.display());

    let dst = tmp.path().join("no_links");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-a", "--no-links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();
    assert!(dst.join("dir/file").exists());
    assert!(!dst.join("link").exists());

    let dst = tmp.path().join("no_perms");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-a", "--no-perms", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();
    let meta = fs::symlink_metadata(dst.join("dir/file")).unwrap();
    assert_ne!(meta.permissions().mode() & 0o777, 0o666);

    let dst = tmp.path().join("no_times");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-a", "--no-times", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();
    let meta = fs::symlink_metadata(dst.join("dir/file")).unwrap();
    assert_ne!(
        FileTime::from_last_modification_time(&meta).unix_seconds(),
        1_234_567
    );

    let dst = tmp.path().join("no_group");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-a", "--no-group", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();
    let meta = fs::symlink_metadata(dst.join("dir/file")).unwrap();
    assert_ne!(meta.gid(), 43);

    let dst = tmp.path().join("no_owner");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-a", "--no-owner", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();
    let meta = fs::symlink_metadata(dst.join("dir/file")).unwrap();
    assert_ne!(meta.uid(), 42);

    let dst = tmp.path().join("no_devices");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-a", "--no-devices", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();
    assert!(!dst.join("dev").exists());
    assert!(fs::symlink_metadata(dst.join("fifo"))
        .unwrap()
        .file_type()
        .is_fifo());

    let dst = tmp.path().join("no_specials");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-a", "--no-specials", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();
    assert!(fs::symlink_metadata(dst.join("dev"))
        .unwrap()
        .file_type()
        .is_char_device());
    assert!(!dst.join("fifo").exists());
}
