// tests/specials_parity.rs
use assert_cmd::prelude::*;
use std::process::Command;
use std::str;

#[test]
fn specials_help_line_matches_rsync() {
    let rsync_output = Command::new("rsync").arg("--help").output().unwrap();
    assert!(rsync_output.status.success());
    let oc_output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();
    assert!(oc_output.status.success());

    let rsync_line = str::from_utf8(&rsync_output.stdout)
        .unwrap()
        .lines()
        .find(|l| l.contains("--specials"))
        .unwrap()
        .trim();
    let oc_line = str::from_utf8(&oc_output.stdout)
        .unwrap()
        .lines()
        .find(|l| l.contains("--specials"))
        .unwrap()
        .trim();
    assert_eq!(oc_line, rsync_line);
}

#[test]
fn specials_flag_parses() {
    Command::new("rsync")
        .args(["--specials", "--version"])
        .assert()
        .success();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--specials", "--version"])
        .assert()
        .success();
}
