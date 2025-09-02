use assert_cmd::Command;
use std::fs;

#[test]
fn help_matches_rsync() {
    let expected = fs::read_to_string("tests/fixtures/rsync-help.txt").unwrap();
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("COLUMNS", "80")
        .arg("--help")
        .output()
        .unwrap();
    let got = String::from_utf8_lossy(&output.stdout);
    assert_eq!(got, expected);
}
