// crates/cli/tests/cli_parity.rs
use assert_cmd::Command;
use oc_rsync_cli::{cli_command, render_help};
use std::path::Path;
use tempfile::tempdir;

#[test]
fn archive_flag_matches_upstream() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    let src_path = src.path();
    let dst_path = dst.path();
    let matches = cli_command()
        .try_get_matches_from([
            "oc-rsync",
            "-a",
            "-n",
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
        ])
        .unwrap();
    assert!(matches.get_flag("archive"));

    let matches = cli_command()
        .try_get_matches_from([
            "oc-rsync",
            "--archive",
            "-n",
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
        ])
        .unwrap();
    assert!(matches.get_flag("archive"));
}

#[test]
fn combined_flags_match_upstream() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    let src_path = src.path();
    let dst_path = dst.path();
    let matches = cli_command()
        .try_get_matches_from([
            "oc-rsync",
            "-avz",
            "-n",
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
        ])
        .unwrap();
    assert!(matches.get_flag("archive"));
    assert!(matches.get_flag("compress"));
    assert_eq!(matches.get_count("verbose"), 1);

    let matches = cli_command()
        .try_get_matches_from([
            "oc-rsync",
            "-a",
            "-v",
            "-z",
            "-n",
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
        ])
        .unwrap();
    assert!(matches.get_flag("archive"));
    assert!(matches.get_flag("compress"));
    assert_eq!(matches.get_count("verbose"), 1);
}

#[test]
fn partial_progress_alias_matches_upstream() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    let src_path = src.path();
    let dst_path = dst.path();
    let matches = cli_command()
        .try_get_matches_from([
            "oc-rsync",
            "-P",
            "-n",
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
        ])
        .unwrap();
    assert!(matches.get_flag("partial_progress"));

    let matches = cli_command()
        .try_get_matches_from([
            "oc-rsync",
            "--partial",
            "--progress",
            "-n",
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
        ])
        .unwrap();
    assert!(matches.get_flag("partial"));
    assert!(matches.get_flag("progress"));
}

#[test]
fn dparam_flag_matches_upstream() {
    let help_output = include_str!("../resources/rsync-help-80.txt");
    assert!(help_output.contains("--dparam"));

    let matches = cli_command()
        .try_get_matches_from(["oc-rsync", "--daemon", "--dparam=pidfile=/dev/null"])
        .unwrap();
    let params: Vec<(String, String)> = matches
        .get_many::<(String, String)>("dparam")
        .unwrap()
        .cloned()
        .collect();
    assert!(params.contains(&("pidfile".into(), "/dev/null".into())));
}

#[test]
fn no_option_alias_matches_upstream() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    let src_path = src.path();
    let dst_path = dst.path();
    let matches = cli_command()
        .try_get_matches_from([
            "oc-rsync",
            "-a",
            "--no-perms",
            "-n",
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
        ])
        .unwrap();
    assert!(matches.get_flag("archive"));
    assert!(matches.get_flag("no-perms"));
}

#[test]
fn help_usage_matches_upstream() {
    let help = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/rsync-help-80.txt"),
    )
    .unwrap();
    let upstream_usage = help
        .lines()
        .find(|l| l.starts_with("Usage:"))
        .unwrap()
        .to_string();
    let ours = render_help(&cli_command());
    let our_usage = ours
        .lines()
        .find(|l| l.starts_with("Usage:"))
        .unwrap()
        .to_string();
    assert_eq!(our_usage, upstream_usage);
}

#[test]
fn misuse_matches_upstream() {
    let golden = include_str!("../../../tests/golden/cli_parity/misuse.stderr");
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--bogus")
        .output()
        .unwrap();
    let ours_stderr = String::from_utf8_lossy(&ours.stderr).to_string();
    let golden_lines: Vec<_> = golden.lines().collect();
    let our_lines: Vec<_> = ours_stderr.lines().collect();
    assert_eq!(ours.status.code(), Some(1));
    assert_eq!(our_lines.get(0), golden_lines.get(0));
    assert!(our_lines
        .get(1)
        .map_or(false, |l| l.starts_with(golden_lines[1])));
}
