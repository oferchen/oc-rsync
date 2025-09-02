// bin/oc-rsync/tests/version.rs
use assert_cmd::Command;
use protocol::SUPPORTED_PROTOCOLS;

fn version_output() -> String {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--version")
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn prints_three_lines() {
    let out = version_output();
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains(env!("CARGO_PKG_VERSION")));
    assert!(lines[0].contains(&SUPPORTED_PROTOCOLS[0].to_string()));
    assert!(lines[1].contains(env!("RSYNC_UPSTREAM_VER")));
    assert!(lines[2].contains(env!("BUILD_REVISION")));
    assert!(lines[2].contains(env!("OFFICIAL_BUILD")));
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
fn build_info_file_has_expected_values() {
    let info = std::fs::read_to_string("../../docs/build_info.md").unwrap();
    assert!(info.contains(env!("RSYNC_UPSTREAM_VER")));
    assert!(info.contains(env!("BUILD_REVISION")));
    assert!(info.contains(env!("OFFICIAL_BUILD")));
}
