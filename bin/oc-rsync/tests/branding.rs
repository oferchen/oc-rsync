// bin/oc-rsync/tests/branding.rs
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn errors_use_program_name() {
    std::env::set_var("PROGRAM_NAME", "myrsync");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--bogus")
        .assert()
        .failure()
        .stderr(contains("myrsync:"))
        .stderr(contains("myrsync error:"));
    std::env::remove_var("PROGRAM_NAME");
}
