// tests/oc_rsyncd_version.rs
use assert_cmd::Command;
use protocol::SUPPORTED_PROTOCOLS;

fn version_output() -> String {
    let output = Command::cargo_bin("oc-rsyncd")
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
    assert!(lines[1].contains(option_env!("RSYNC_UPSTREAM_VER").unwrap_or("unknown")));
    assert!(lines[2].contains(option_env!("BUILD_REVISION").unwrap_or("unknown")));
    assert!(lines[2].contains(option_env!("OFFICIAL_BUILD").unwrap_or("unofficial")));
}

#[test]
fn output_is_immutable() {
    let first = version_output();
    let second = version_output();
    assert_eq!(first, second);
}

#[test]
fn exit_code_is_zero() {
    Command::cargo_bin("oc-rsyncd")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
}

#[test]
fn quiet_suppresses_output() {
    let output = Command::cargo_bin("oc-rsyncd")
        .unwrap()
        .args(["--version", "--quiet"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}
