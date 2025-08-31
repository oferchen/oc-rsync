use oc_rsync_cli::cli_command;
use std::process::Command;
use tempfile::tempdir;

// Integration tests to ensure our CLI parsing aligns with upstream rsync for
// critical flags, including short/long aliases and combined short flags.

#[test]
fn archive_flag_matches_upstream() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    let src_path = src.path();
    let dst_path = dst.path();

    // upstream short flag
    let status = Command::new("rsync")
        .args(["-a", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .expect("rsync not installed");
    assert!(status.success());

    // our parser short flag
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

    // upstream long flag
    let status = Command::new("rsync")
        .args(["--archive", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .expect("rsync not installed");
    assert!(status.success());

    // our parser long flag
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

    // upstream combined flags
    let status = Command::new("rsync")
        .args(["-avz", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .expect("rsync not installed");
    assert!(status.success());

    // our parser combined flags
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

    // upstream separate flags
    let status = Command::new("rsync")
        .args(["-a", "-v", "-z", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .expect("rsync not installed");
    assert!(status.success());

    // our parser separate flags
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

    // upstream short alias -P
    let status = Command::new("rsync")
        .args(["-P", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .expect("rsync not installed");
    assert!(status.success());

    // our parser for -P
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

    // upstream long form --partial --progress
    let status = Command::new("rsync")
        .args(["--partial", "--progress", "-n"])
        .arg(src_path)
        .arg(dst_path)
        .status()
        .expect("rsync not installed");
    assert!(status.success());

    // our parser long form
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
