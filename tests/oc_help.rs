use assert_cmd::Command;
use std::fs;
use std::io::Write;
use std::process::{Command as StdCommand, Stdio};

#[test]
fn help_output_matches_golden() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .arg("--help")
        .output()
        .unwrap();

    let mut child = StdCommand::new("scripts/sanitize-banner.sh")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&output.stdout)
        .unwrap();
    let sanitized = child.wait_with_output().unwrap().stdout;

    let expected = fs::read("tests/golden/oc/help.txt").unwrap();
    assert_eq!(sanitized, expected, "`--help` output mismatch");
}
