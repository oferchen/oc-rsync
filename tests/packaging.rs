// tests/packaging.rs
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
            .any(|l| l.trim() == "packaging/examples/oc-rsyncd.conf"),
        "example config missing from package list:\n{}",
        listing
    );
    assert!(
        listing
            .lines()
            .any(|l| l.trim() == "packaging/systemd/oc-rsyncd.conf"),
        "systemd example config missing from package list:\n{}",
        listing
    );
    assert!(
        listing.lines().any(|l| l.trim() == "man/oc-rsyncd.8"),
        "daemon man page missing from package list:\n{}",
        listing
    );
}

#[test]
#[ignore]
fn service_unit_matches_spec() {
    let unit = std::fs::read_to_string("packaging/systemd/oc-rsyncd.service")
        .expect("failed to read service unit");
    for expected in [
        "ProtectSystem=strict",
        "ProtectHome=true",
        "Restart=on-failure",
        "RestartSec=2s",
        "ExecStart=/usr/local/bin/oc-rsync --daemon --no-detach --config=/etc/oc-rsyncd.conf",
        "CapabilityBoundingSet=CAP_NET_BIND_SERVICE CAP_DAC_READ_SEARCH CAP_FOWNER CAP_CHOWN CAP_DAC_OVERRIDE",
        "AmbientCapabilities=CAP_NET_BIND_SERVICE CAP_DAC_READ_SEARCH CAP_FOWNER CAP_CHOWN CAP_DAC_OVERRIDE",
        "RestrictNamespaces=yes",
        "RuntimeDirectory=oc-rsyncd",
        "LogsDirectory=oc-rsyncd",
        "StateDirectory=oc-rsyncd",
        "ConfigurationDirectory=oc-rsyncd",
        "ExecStart=/usr/local/bin/oc-rsyncd --no-detach --config=/etc/oc-rsyncd.conf",
        "Documentation=man:oc-rsyncd(8) man:oc-rsyncd.conf(5) man:oc-rsync(1)",
    ] {
        assert!(
            unit.lines().any(|l| l.trim() == expected),
            "missing `{}` in service unit",
            expected
        );
    }
}
