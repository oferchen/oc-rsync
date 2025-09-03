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

    assert_eq!(oc_output.stdout, up_output.stdout);
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

    assert_eq!(oc_output.stdout, up_output.stdout);
}
