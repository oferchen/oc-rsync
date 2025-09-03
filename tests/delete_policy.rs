// tests/delete_policy.rs

use assert_cmd::Command;
#[cfg(unix)]
use nix::unistd::Uid;
use std::fs;
use tempfile::tempdir;

#[test]
fn max_delete_aborts_after_limit() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("keep.txt"), b"keep").unwrap();
    fs::write(dst.join("old1.txt"), b"old").unwrap();
    fs::write(dst.join("old2.txt"), b"old").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--delete",
            "--max-delete=1",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let mut remaining = 0;
    if dst.join("old1.txt").exists() {
        remaining += 1;
    }
    if dst.join("old2.txt").exists() {
        remaining += 1;
    }
    assert_eq!(remaining, 1);
}

#[test]
fn max_delete_allows_within_limit() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("keep.txt"), b"keep").unwrap();
    fs::write(dst.join("old1.txt"), b"old").unwrap();
    fs::write(dst.join("old2.txt"), b"old").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--delete",
            "--max-delete=2",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("old1.txt").exists());
    assert!(!dst.join("old2.txt").exists());
}

#[test]
fn delete_missing_args_removes_destination() {
    let dir = tempdir().unwrap();
    let dst = dir.path().join("orphan.txt");
    fs::write(&dst, b"old").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--delete-missing-args",
            "missing.txt",
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.exists());
}

#[test]
fn remove_source_files_via_cli() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.txt"), b"hi").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--remove-source-files",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("file.txt").exists());
    assert!(!src.join("file.txt").exists());
    assert!(src.exists());
}

#[cfg(unix)]
#[test]
fn ignore_errors_allows_deletion_failure() {
    use std::os::unix::fs::PermissionsExt;

    if Uid::effective().is_root() {
        eprintln!("skipping: requires non-root privileges");
        return;
    }

    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(dst.join("old.txt"), b"old").unwrap();
    fs::set_permissions(&dst, fs::Permissions::from_mode(0o555)).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--recursive", "--delete", &src_arg, dst.to_str().unwrap()])
        .assert()
        .failure();
    assert!(dst.join("old.txt").exists());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--delete",
            "--ignore-errors",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("old.txt").exists());

    fs::set_permissions(&dst, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn force_deletes_non_empty_dirs() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(dst.join("sub")).unwrap();
    fs::write(dst.join("sub/file.txt"), b"hi").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--delete",
            "--force",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("sub").exists());
}
