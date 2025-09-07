// tests/dry_run.rs

use assert_cmd::Command;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[test]
fn dry_run_deletions_match_rsync() {
    let check = StdCommand::new("rsync")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok();
    if let Some(status) = check {
        assert!(status.success());
    } else {
        eprintln!("skipping test: rsync not installed");
        return;
    }

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
    let rsync = StdCommand::new("rsync")
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
    let our_out = String::from_utf8(ours.stdout).unwrap();
    let our_lines: Vec<_> = our_out
        .lines()
        .filter(|l| l.starts_with("deleting "))
        .collect();
    let up_out = String::from_utf8(rsync.stdout).unwrap();
    let up_lines: Vec<_> = up_out
        .lines()
        .filter(|l| l.starts_with("deleting "))
        .collect();
    assert_eq!(rsync.status.code(), ours.status.code());
    assert_eq!(up_lines, our_lines);
}

#[test]
fn dry_run_errors_match_rsync() {
    let check = StdCommand::new("rsync")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok();
    if let Some(status) = check {
        assert!(status.success());
    } else {
        eprintln!("skipping test: rsync not installed");
        return;
    }

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
    let up = StdCommand::new("rsync")
        .current_dir(tmp.path())
        .env("LC_ALL", "C")
        .args(["--dry-run", "missing.txt", dst.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(up.status.code(), ours.status.code());
    assert_eq!(up.stdout, ours.stdout);
    assert_eq!(up.stderr, ours.stderr);
}
