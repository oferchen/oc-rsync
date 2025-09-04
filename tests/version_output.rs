// tests/version_output.rs
use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use std::fs;

#[test]
fn version_matches_banner() {
    let expected = oc_rsync_cli::version::version_banner();
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

#[test]
fn daemon_version_matches_banner() {
    let expected = oc_rsync_cli::version::version_banner();
    let output = Command::cargo_bin("oc-rsyncd")
        .unwrap()
        .env("OC_RSYNC_BIN", cargo_bin("oc-rsync"))
        .arg("--version")
        .output()
        .unwrap();
    let got = String::from_utf8_lossy(&output.stdout);
    assert_eq!(got, expected);
}

#[test]
fn version_matches_golden() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .arg("--version")
        .output()
        .unwrap();

    let mut parts = output.stdout.splitn(6, |b| *b == b'\n');
    for _ in 0..5 {
        parts.next();
    }
    let actual = parts.next().unwrap_or_default();
    let expected = fs::read("tests/golden/version/oc-rsync.version").unwrap();
    assert_eq!(actual, expected, "version output does not match golden");
}
