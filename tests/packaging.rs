use std::process::Command;

#[test]
fn packaging_includes_service_unit() {
    let output = Command::new("cargo")
        .args(["package", "--list", "--allow-dirty", "--no-verify"])
        .output()
        .expect("failed to run cargo package");
    assert!(output.status.success(), "cargo package failed");
    let listing = String::from_utf8_lossy(&output.stdout);
    assert!(
        listing
            .lines()
            .any(|l| l.trim() == "packaging/systemd/oc-rsyncd.service"),
        "service unit missing from package list:\n{}",
        listing
    );
    assert!(
        listing
            .lines()
            .any(|l| l.trim() == "packaging/rsyncd.conf.example"),
        "example config missing from package list:\n{}",
        listing
    );
}
