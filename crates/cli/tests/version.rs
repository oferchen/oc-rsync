// crates/cli/tests/version.rs
use oc_rsync_cli::version_string;
use std::process::{Command, Stdio};

macro_rules! require_rsync {
    () => {
        let rsync = Command::new("rsync")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok();
        if rsync.is_none() {
            eprintln!("skipping test: rsync not installed");
            return;
        }
        assert!(rsync.is_some());
    };
}

#[test]
fn version_matches_upstream() {
    require_rsync!();
    let expected = Command::new("rsync").arg("--version").output().unwrap();
    let expected = String::from_utf8_lossy(&expected.stdout);
    let ours = version_string();
    assert_eq!(ours, expected);
}
