// tests/delete_policy.rs

use assert_cmd::Command;
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
            "--local",
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
            "--local",
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
