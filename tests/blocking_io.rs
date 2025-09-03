// tests/blocking_io.rs
use assert_cmd::Command;
use std::process::Command as StdCommand;

#[test]
fn version_matches_upstream_nonblocking() {
    let oc_output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--version")
        .output()
        .unwrap();
    assert!(oc_output.status.success());

    let up_output = StdCommand::new("rsync").arg("--version").output().unwrap();
    assert!(up_output.status.success());
    let oc_str = String::from_utf8(oc_output.stdout).unwrap();
    let up_str = String::from_utf8(up_output.stdout).unwrap();
    let oc_lines: Vec<_> = oc_str.lines().collect();
    let up_lines: Vec<_> = up_str.lines().collect();
    let tail = oc_lines.len() - 3;
    assert_eq!(&oc_lines[3..], &up_lines[1..1 + tail]);
}

#[test]
fn version_matches_upstream_blocking() {
    let oc_output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--blocking-io", "--version"])
        .output()
        .unwrap();
    assert!(oc_output.status.success());

    let up_output = StdCommand::new("rsync")
        .args(["--blocking-io", "--version"])
        .output()
        .unwrap();
    assert!(up_output.status.success());
    let oc_str = String::from_utf8(oc_output.stdout).unwrap();
    let up_str = String::from_utf8(up_output.stdout).unwrap();
    let oc_lines: Vec<_> = oc_str.lines().collect();
    let up_lines: Vec<_> = up_str.lines().collect();
    let tail = oc_lines.len() - 3;
    assert_eq!(&oc_lines[3..], &up_lines[1..1 + tail]);
}
