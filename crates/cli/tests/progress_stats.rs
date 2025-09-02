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
        .args(["-r", "--progress"])
        .arg(src.join("a.txt"))
        .arg(&dst_up)
        .output()
        .unwrap();
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--progress", src.join("a.txt").to_str().unwrap(), dst_ours.to_str().unwrap()])
        .output()
        .unwrap();

    let up_line = String::from_utf8_lossy(&up.stderr);
    let up_line = up_line.split('\r').last().unwrap().trim();
    let up_fields: Vec<&str> = up_line.split_whitespace().take(3).collect();

    let our_line = String::from_utf8_lossy(&ours.stderr);
    let our_line = our_line.split('\r').last().unwrap().trim();
    let our_fields: Vec<&str> = our_line.split_whitespace().take(3).collect();

    assert_eq!(our_fields, up_fields);
    insta::assert_snapshot!("progress_parity", our_fields.join(" "));
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
        .args(["-r", "--stats"])
        .arg(format!("{}/", src.display()))
        .arg(&dst_up)
        .output()
        .unwrap();
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--stats", format!("{}/", src.display()).as_str(), dst_ours.to_str().unwrap()])
        .output()
        .unwrap();

    let up_stdout = String::from_utf8_lossy(&up.stdout);
    let up_stats: Vec<&str> = up_stdout
        .lines()
        .filter(|l| l.starts_with("Number of regular files transferred")
            || l.starts_with("Number of deleted files")
            || l.starts_with("Total transferred file size"))
        .collect();

    let our_stdout = String::from_utf8_lossy(&ours.stdout);
    let our_stats: Vec<&str> = our_stdout.lines().collect();

    assert_eq!(our_stats, up_stats);
    insta::assert_snapshot!("stats_parity", our_stats.join("\n"));
}
