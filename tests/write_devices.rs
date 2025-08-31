use assert_cmd::Command;

#[test]
fn write_devices_flag_parses() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--write-devices", "--help"])
        .assert()
        .success();
}
