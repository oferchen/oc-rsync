// crates/cli/tests/progress_stats.rs
use assert_cmd::Command;
use std::process::{Command as StdCommand, Stdio};
use tempfile::tempdir;

macro_rules! require_rsync {
    () => {
        let rsync = StdCommand::new("rsync")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok();
        if rsync.is_none() {
            eprintln!("skipping test: rsync not installed");
            return;
        }
    };
}

#[test]
fn progress_parity() {
    require_rsync!();
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst_up = dir.path().join("dst_up");
    let dst_ours = dir.path().join("dst_ours");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), b"hello").unwrap();

    let up = StdCommand::new("rsync")
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .args(["-r", "--progress"])
        .arg(format!("{}/", src.display()))
        .arg(&dst_up)
        .output()
        .unwrap();
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .args([
            "--local",
            "--progress",
            format!("{}/", src.display()).as_str(),
            dst_ours.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let up_stderr = String::from_utf8_lossy(&up.stderr).replace('\r', "\n");
    let our_stderr = String::from_utf8_lossy(&ours.stderr).replace('\r', "\n");

    assert_eq!(our_stderr, up_stderr);
    insta::assert_snapshot!("progress_parity", our_stderr);
}

#[test]
fn stats_parity() {
    require_rsync!();
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst_up = dir.path().join("dst_up");
    let dst_ours = dir.path().join("dst_ours");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), b"hello").unwrap();

    let up = StdCommand::new("rsync")
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .args(["-r", "--stats"])
        .arg(format!("{}/", src.display()))
        .arg(&dst_up)
        .output()
        .unwrap();
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .args([
            "--local",
            "--stats",
            format!("{}/", src.display()).as_str(),
            dst_ours.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let up_stdout = String::from_utf8_lossy(&up.stdout);
    let up_stats: Vec<&str> = up_stdout
        .lines()
        .filter(|l| {
            l.starts_with("Number of regular files transferred")
                || l.starts_with("Number of deleted files")
                || l.starts_with("Total transferred file size")
        })
        .collect();

    let our_stdout = String::from_utf8_lossy(&ours.stdout);
    let our_stats: Vec<&str> = our_stdout.lines().collect();

    assert_eq!(our_stats, up_stats);
    insta::assert_snapshot!("stats_parity", our_stats.join("\n"));
}
