// crates/cli/tests/iconv.rs
use assert_cmd::Command;
use oc_rsync_cli::{cli_command, parse_iconv};
use std::process::Command as StdCommand;
use tempfile::tempdir;

macro_rules! require_rsync {
    () => {
        let rsync = StdCommand::new("rsync")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .ok();
        if rsync.is_none() {
            eprintln!("skipping test: rsync not installed");
            return;
        }
    };
}

#[test]
fn iconv_help_matches_upstream() {
    require_rsync!();
    let ours = cli_command().render_help().to_string();
    let our_line = ours.lines().find(|l| l.contains("--iconv")).unwrap().trim();

    let output = StdCommand::new("rsync").arg("--help").output().unwrap();
    let help = String::from_utf8(output.stdout).unwrap();
    let upstream_line = help.lines().find(|l| l.contains("--iconv")).unwrap().trim();

    assert_eq!(our_line, upstream_line);
}

#[test]
fn invalid_iconv_spec_errors() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--iconv=utf8,bogus",
            src.path().to_str().unwrap(),
            dst.path().to_str().unwrap(),
        ])
        .assert()
        .failure();
}

#[test]
fn iconv_converts_encodings() {
    let cv = parse_iconv("utf-8,iso8859-1").unwrap();
    assert_eq!(cv.encode_remote("é"), vec![0xe9]);
    let local = cv.to_local(&[0xe9]);
    assert_eq!(String::from_utf8(local).unwrap(), "é");
}
