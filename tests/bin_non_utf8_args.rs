// tests/bin_non_utf8_args.rs
#![cfg(unix)]
use assert_cmd::Command;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

fn non_utf8_arg() -> OsString {
    OsString::from_vec(b"\xff".to_vec())
}

#[test]
fn version_handles_non_utf8_arg() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg(non_utf8_arg())
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn quiet_suppresses_version_with_non_utf8_arg() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg(non_utf8_arg())
        .arg("--version")
        .arg("--quiet")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}
