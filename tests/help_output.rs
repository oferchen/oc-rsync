// tests/help_output.rs
use assert_cmd::Command;
use std::collections::HashSet;
use std::fs;

#[test]
fn dump_help_body_lists_unique_options() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--dump-help-body")
        .output()
        .unwrap();

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
fn help_matches_snapshot() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("COLUMNS", "80")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .arg("--help")
        .output()
        .unwrap();

    let mut parts = output.stdout.splitn(4, |b| *b == b'\n');
    parts.next();
    parts.next();
    parts.next();
    let actual = parts.next().unwrap_or_default();
    let expected = fs::read("tests/golden/help/oc-rsync.help").unwrap();
    assert_eq!(actual, expected, "help output does not match snapshot");
}
