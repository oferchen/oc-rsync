// tests/version_output.rs
use assert_cmd::Command;

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
