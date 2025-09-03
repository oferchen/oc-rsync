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
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
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
    let our_out = String::from_utf8(ours.stdout).unwrap();
    let our_lines: Vec<_> = our_out
        .lines()
        .filter(|l| l.starts_with("deleting "))
        .collect();

    let rsync = StdCommand::new("rsync")
        .args([
            "--recursive",
            "--delete",
            "--dry-run",
            "--verbose",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .output()
        .expect("rsync not installed");
    let rsync_out = String::from_utf8(rsync.stdout).unwrap();
    let rsync_lines: Vec<_> = rsync_out
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

    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .current_dir(tmp.path())
        .args(["--dry-run", "missing.txt", dst.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(ours.status.code(), Some(23));
    let ours_err = String::from_utf8(ours.stderr).unwrap();

    let rsync = StdCommand::new("rsync")
        .current_dir(tmp.path())
        .args(["--dry-run", "missing.txt", dst.to_str().unwrap()])
        .output()
        .expect("rsync not installed");
    assert_eq!(rsync.status.code(), Some(23));
    let rsync_err = String::from_utf8(rsync.stderr).unwrap();

    let mut expected_lines = rsync_err.lines();
    let mut our_lines = ours_err.lines();
    assert_eq!(expected_lines.next(), our_lines.next());
    let exp_second = expected_lines.next().unwrap();
    let our_second = our_lines.next().unwrap();
    let exp_prefix = exp_second.split(" at ").next().unwrap();
    let our_prefix = our_second.split(" at ").next().unwrap();
    assert_eq!(exp_prefix, our_prefix);
}
