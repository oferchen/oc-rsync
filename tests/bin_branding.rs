// tests/bin_branding.rs
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn errors_use_program_name() {
    unsafe { std::env::set_var("OC_RSYNC_BRAND_NAME", "myrsync") };
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--bogus")
        .assert()
        .failure()
        .stderr(contains("myrsync:"))
        .stderr(contains("myrsync error:"));
    unsafe { std::env::remove_var("OC_RSYNC_BRAND_NAME") };
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
