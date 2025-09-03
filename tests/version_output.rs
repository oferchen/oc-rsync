// tests/version_output.rs
use assert_cmd::Command;
use std::fs;

#[test]
fn version_matches_rsync() {
    let expected = fs::read_to_string("tests/fixtures/rsync-version.txt").unwrap();
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .arg("--version")
        .output()
        .unwrap();
    let got = String::from_utf8_lossy(&output.stdout);
    assert_eq!(got, expected);
}
