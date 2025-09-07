// tests/dry_run.rs

use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;
mod common;
use common::{normalize_paths, read_golden};

#[test]
fn dry_run_deletions_match_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(dst.join("old.txt"), b"old").unwrap();

    let src_arg = format!("{}/", src.display());
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .args([
            "--recursive",
            "--delete",
            "--dry-run",
            "--verbose",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let (exp_stdout, _exp_stderr, exp_exit) = read_golden("dry_run/deletions");
    let expected_str = String::from_utf8(exp_stdout).unwrap();
    let expected: Vec<_> = expected_str.lines().collect();
    let ours_stdout = String::from_utf8(ours.stdout).unwrap();
    let ours_lines: Vec<_> = ours_stdout
        .lines()
        .filter(|l| l.starts_with("deleting "))
        .collect();
    assert_eq!(ours.status.code(), Some(exp_exit));
    assert_eq!(ours_lines, expected);
}

#[test]
fn dry_run_errors_match_rsync() {
    let tmp = tempdir().unwrap();
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .current_dir(tmp.path())
        .env("LC_ALL", "C")
        .args(["--dry-run", "missing.txt", dst.to_str().unwrap()])
        .output()
        .unwrap();

    let (exp_stdout, exp_stderr, exp_exit) = read_golden("dry_run/errors");
    let our_stdout = normalize_paths(&ours.stdout, tmp.path());
    let our_stderr = normalize_paths(&ours.stderr, tmp.path());

    assert_eq!(ours.status.code(), Some(exp_exit));
    assert_eq!(our_stdout, String::from_utf8(exp_stdout).unwrap());
    assert_eq!(our_stderr, String::from_utf8(exp_stderr).unwrap());
}
