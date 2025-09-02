// tests/dry_run.rs

use assert_cmd::Command;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[test]
fn dry_run_deletions_match_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(dst.join("old.txt"), b"old").unwrap();

    let src_arg = format!("{}/", src.display());
    let rsync = StdCommand::new("rsync")
        .args([
            "-av",
            "--delete",
            "--dry-run",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let ours = Command::cargo_bin("oc-rsync")
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
        .output()
        .unwrap();
    assert_eq!(rsync.status.code(), ours.status.code());
    let rsync_out = String::from_utf8(rsync.stdout).unwrap();
    let rsync_lines: Vec<_> = rsync_out
        .lines()
        .filter(|l| l.starts_with("deleting "))
        .collect();
    let our_out = String::from_utf8(ours.stdout).unwrap();
    let our_lines: Vec<_> = our_out
        .lines()
        .filter(|l| l.starts_with("deleting "))
        .collect();
    assert_eq!(rsync_lines, our_lines);
}

#[test]
fn dry_run_errors_match_rsync() {
    let tmp = tempdir().unwrap();
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    let rsync = StdCommand::new("rsync")
        .current_dir(tmp.path())
        .args(["--dry-run", "missing.txt", dst.to_str().unwrap()])
        .output()
        .unwrap();
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .current_dir(tmp.path())
        .args(["--local", "--dry-run", "missing.txt", dst.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(rsync.status.code(), ours.status.code());
    let rsync_err = String::from_utf8(rsync.stderr).unwrap();
    let ours_err = String::from_utf8(ours.stderr).unwrap();
    let mut rs_lines = rsync_err.lines();
    let mut our_lines = ours_err.lines();
    assert_eq!(rs_lines.next(), our_lines.next());
    let rs_second = rs_lines.next().unwrap().split(" (code 23)").next().unwrap();
    let our_second = our_lines.next().unwrap();
    assert_eq!(format!("{rs_second} (code 23)"), our_second);
}
