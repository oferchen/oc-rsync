// tests/interop/outstanding_flags.rs
#![cfg(all(unix, feature = "interop"))]

use assert_cmd::Command;
use std::process::Command as StdCommand;

const FLAGS: &[&str] = &[
    "--config=/dev/null",
    "--copy-as=0:0",
    "--early-input",
    "--fake-super",
    "--fsync",
    "--fuzzy",
    "--groupmap=0:0",
    "--ignore-errors",
    "--ignore-missing-args",
    "--ignore-times",
    "--info=progress2",
    "--max-size=1",
    "--min-size=1",
    "--modify-window=1",
    "--old-args",
    "--old-d",
    "--old-dirs",
    "--open-noatime",
    "--outbuf=L",
    "--progress",
    "-P",
    "--protocol=31",
    "--relative",
    "--secluded-args",
    "--server",
    "--sockopts=SO_KEEPALIVE",
    "--specials",
    "--stats",
    "--stop-after=1",
    "--stop-at=1",
    "--super",
    "--temp-dir=/tmp",
    "--timeout=1",
    "--trust-sender",
    "--update",
    "--usermap=0:0",
    "--verbose",
    "--xattrs",
];

#[test]
#[ignore = "requires rsync"]
fn outstanding_flags_help_matches_upstream() {
    for flag in FLAGS {
        let ours = Command::cargo_bin("oc-rsync")
            .unwrap()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .args([*flag, "--help"])
            .output()
            .unwrap();
        assert!(ours.status.success(), "oc-rsync failed for flag {flag}");

        let upstream = StdCommand::new("rsync")
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .args([*flag, "--help"])
            .output()
            .expect("failed to run rsync");
        assert!(upstream.status.success(), "rsync failed for flag {flag}");

        assert_eq!(
            ours.stdout, upstream.stdout,
            "help output mismatch for flag {flag}"
        );
    }
}
