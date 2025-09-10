// tests/help_output.rs
use std::collections::HashSet;
use std::fs;
mod common;
use common::oc_cmd;

#[test]
fn dump_help_body_lists_unique_options() {
    let output = oc_cmd()
        .arg("--dump-help-body")
        .assert()
        .success()
        .get_output()
        .clone();

    let mut seen = HashSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, '\t');
        let flag = parts.next().unwrap();
        let desc = parts.next().unwrap_or("").trim();
        assert!(flag.starts_with("--"), "non-long option {flag}");
        assert!(!desc.is_empty(), "missing description for {flag}");
        assert!(seen.insert(flag.to_string()), "duplicate flag {flag}");
    }
}
#[test]
fn help_output_matches_golden() {
    let output = oc_cmd()
        .env("COLUMNS", "80")
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .clone();

    let expected = fs::read("tests/golden/help/oc-rsync.help").unwrap();
    assert_eq!(output.stdout, expected, "`--help` output mismatch");
}

#[test]
fn dump_help_body_60_matches_golden() {
    let output = oc_cmd()
        .env("COLUMNS", "60")
        .arg("--dump-help-body")
        .assert()
        .success()
        .get_output()
        .clone();

    let expected = fs::read("tests/golden/help/oc-rsync.dump-help-body.60").unwrap();
    assert_eq!(output.stdout, expected, "dump-help-body width 60 mismatch");
}

#[test]
fn dump_help_body_100_matches_golden() {
    let output = oc_cmd()
        .env("COLUMNS", "100")
        .arg("--dump-help-body")
        .assert()
        .success()
        .get_output()
        .clone();

    let expected = fs::read("tests/golden/help/oc-rsync.dump-help-body.100").unwrap();
    assert_eq!(output.stdout, expected, "dump-help-body width 100 mismatch");
}

#[test]
fn unknown_option_matches_snapshot() {
    let output = oc_cmd()
        .arg("--bad-option")
        .arg("src")
        .arg("dst")
        .assert()
        .failure()
        .get_output()
        .clone();

    let expected = fs::read("tests/golden/help/oc-rsync.bad-option.stderr").unwrap();
    assert_eq!(output.stderr, expected, "unknown option stderr mismatch");
}

#[test]
fn invalid_numeric_value_matches_snapshot() {
    let output = oc_cmd()
        .arg("--timeout=abc")
        .arg("src")
        .arg("dst")
        .assert()
        .failure()
        .get_output()
        .clone();

    let expected = fs::read("tests/golden/help/oc-rsync.invalid-timeout.stderr").unwrap();
    assert_eq!(output.stderr, expected, "invalid timeout stderr mismatch");
}
