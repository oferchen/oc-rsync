// tests/interop/help.rs
#![cfg(unix)]

use assert_cmd::Command;
use std::process::Command as StdCommand;

fn normalize(bytes: &[u8]) -> &[u8] {
    let needle = b"rsync comes with ABSOLUTELY NO WARRANTY.";
    let pos = bytes
        .windows(needle.len())
        .position(|w| w == needle)
        .expect("warranty line not found");
    &bytes[pos..]
}

#[test]
#[ignore = "requires rsync"]
fn help_output_matches_upstream() {
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .arg("--help")
        .output()
        .unwrap();
    assert!(ours.status.success());

    let upstream = StdCommand::new("rsync")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .arg("--help")
        .output()
        .expect("failed to run rsync");
    assert!(upstream.status.success());

    assert_eq!(normalize(&ours.stdout), normalize(&upstream.stdout));
}

