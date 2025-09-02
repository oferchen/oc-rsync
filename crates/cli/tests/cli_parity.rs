// crates/cli/tests/cli_parity.rs
use oc_rsync_cli::cli_command;
use std::process::{Command, Stdio};
use tempfile::tempdir;

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
fn archive_flag_matches_upstream() {
    require_rsync!();
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    let src_path = src.path();
    let dst_path = dst.path();

    let status = Command::new("rsync")
        .args(["-a", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .unwrap();
    assert!(status.success());

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

    let status = Command::new("rsync")
        .args(["--archive", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .unwrap();
    assert!(status.success());

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
    require_rsync!();
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    let src_path = src.path();
    let dst_path = dst.path();

    let status = Command::new("rsync")
        .args(["-avz", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .unwrap();
    assert!(status.success());

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

    let status = Command::new("rsync")
        .args(["-a", "-v", "-z", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .unwrap();
    assert!(status.success());

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
    require_rsync!();
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    let src_path = src.path();
    let dst_path = dst.path();

    let status = Command::new("rsync")
        .args(["-P", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .unwrap();
    assert!(status.success());

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

    let status = Command::new("rsync")
        .args(["--partial", "--progress", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .unwrap();
    assert!(status.success());

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
    require_rsync!();
    let status = Command::new("rsync")
        .args(["--daemon", "--dparam=pidfile=/dev/null", "--help"])
        .status()
        .unwrap();
    assert!(status.success());

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
    require_rsync!();
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    let src_path = src.path();
    let dst_path = dst.path();

    let status = Command::new("rsync")
        .args(["-a", "--no-perms", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .unwrap();
    assert!(status.success());

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
