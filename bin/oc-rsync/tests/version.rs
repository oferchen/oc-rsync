// bin/oc-rsync/tests/version.rs
use assert_cmd::Command;

fn version_output() -> String {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--version")
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn matches_upstream_output() {
    let out = version_output();
    let expected = include_str!("../../tests/fixtures/rsync-version.txt");
    assert_eq!(out, expected);
}

#[test]
fn output_is_immutable() {
    let first = version_output();
    let second = version_output();
    assert_eq!(first, second);
}

#[test]
fn exit_code_is_zero() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
}

#[test]
fn quiet_suppresses_output() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--version", "--quiet"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn build_info_file_has_expected_values() {
    let info = std::fs::read_to_string(env!("BUILD_INFO_PATH")).unwrap();
    assert!(info.contains(env!("RSYNC_UPSTREAM_VER")));
    assert!(info.contains(env!("BUILD_REVISION")));
    assert!(info.contains(env!("OFFICIAL_BUILD")));
}
