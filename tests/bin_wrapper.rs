// tests/bin_wrapper.rs
use assert_cmd::cargo::{cargo_bin, CommandCargoExt};
use std::process::Command;

#[test]
fn version_matches_daemon() {
    let expected = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--daemon", "--version"])
        .output()
        .unwrap();

    let actual = Command::cargo_bin("oc-rsyncd")
        .unwrap()
        .env("OC_RSYNC_BIN", cargo_bin("oc-rsync"))
        .arg("--version")
        .output()
        .unwrap();

    assert_eq!(actual.stdout, expected.stdout);
    assert_eq!(actual.stderr, expected.stderr);
}

#[test]
fn help_matches_daemon() {
    let expected = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--daemon", "--help"])
        .output()
        .unwrap();

    let actual = Command::cargo_bin("oc-rsyncd")
        .unwrap()
        .env("OC_RSYNC_BIN", cargo_bin("oc-rsync"))
        .arg("--help")
        .output()
        .unwrap();

    assert_eq!(actual.stdout, expected.stdout);
    assert_eq!(actual.stderr, expected.stderr);
}
