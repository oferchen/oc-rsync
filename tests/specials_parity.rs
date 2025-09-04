// tests/specials_parity.rs
use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;
use std::str;

#[test]
fn specials_help_line_matches_rsync() {
    let expected = fs::read_to_string("tests/golden/help/rsync_specials_line.txt")
        .unwrap()
        .trim()
        .to_owned();
    let oc_output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();
    assert!(oc_output.status.success());

    let oc_line = str::from_utf8(&oc_output.stdout)
        .unwrap()
        .lines()
        .find(|l| l.contains("--specials"))
        .unwrap()
        .trim();
    assert_eq!(oc_line, expected);
}

#[test]
fn specials_flag_parses() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--specials", "--version"])
        .assert()
        .success();
}
