// crates/cli/tests/iconv.rs
use assert_cmd::Command;
use oc_rsync_cli::{cli_command, parse_iconv, render_help};
use std::path::Path;
use tempfile::tempdir;

#[test]
fn iconv_help_matches_upstream() {
    let ours = render_help(&cli_command());
    let our_line = ours.lines().find(|l| l.contains("--iconv")).unwrap().trim();

    let help = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/rsync-help-80.txt"),
    )
    .unwrap();
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
