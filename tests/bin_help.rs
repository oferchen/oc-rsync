// tests/bin_help.rs
use assert_cmd::Command;

#[test]
fn exit_code_is_zero() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}
