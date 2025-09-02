// tests/dry_run.rs

use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::tempdir;

#[test]
fn dry_run_reports_deletions() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(dst.join("old.txt"), b"old").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--delete",
            "--dry-run",
            "--verbose",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("deleting old.txt"));

    assert!(dst.join("old.txt").exists());
}

#[test]
fn dry_run_propagates_errors() {
    let tmp = tempdir().unwrap();
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--dry-run", "missing.txt", dst.to_str().unwrap()])
        .assert()
        .code(23);
}
