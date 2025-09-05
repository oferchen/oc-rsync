// tests/bin_branding.rs
use assert_cmd::Command;
use predicates::str::contains;
use serial_test::serial;

fn set_env(key: &str, value: &str) {
    unsafe { std::env::set_var(key, value) }
}

fn remove_env(key: &str) {
    unsafe { std::env::remove_var(key) }
}

#[test]
#[serial]
fn errors_use_program_name() {
    set_env("OC_RSYNC_NAME", "myrsync");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--bogus")
        .assert()
        .failure()
        .stderr(contains("myrsync:"))
        .stderr(contains("myrsync error:"));
    remove_env("OC_RSYNC_NAME");
}

#[test]
fn help_shows_oc_rsync_branding() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("oc-rsync"));
    assert!(stdout.contains("https://github.com/oferchen/oc-rsync"));
    assert!(!stdout.contains("rsync.samba.org"));
}

#[test]
fn version_has_no_upstream_urls() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--version")
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("oc-rsync"));
    assert!(!stdout.contains("rsync.samba.org"));
}
