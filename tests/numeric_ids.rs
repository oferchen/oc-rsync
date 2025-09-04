// tests/numeric_ids.rs

use assert_cmd::Command;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[cfg(unix)]
use nix::unistd::{chown, Gid, Uid};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(unix)]
#[test]
fn numeric_ids_matches_rsync() {
    if !Uid::effective().is_root() {
        eprintln!("skipping numeric_ids_matches_rsync: requires root or CAP_CHOWN",);
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let ours = tmp.path().join("ours");
    let rsync_dst = tmp.path().join("rsync");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&ours).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    let file = src.join("id.txt");
    fs::write(&file, b"ids").unwrap();

    let uid = Uid::from_raw(1);
    let gid = Gid::from_raw(1);
    chown(&file, Some(uid), Some(gid)).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--numeric-ids",
            "--owner",
            "--group",
            &src_arg,
            ours.to_str().unwrap(),
        ])
        .assert()
        .success();

    StdCommand::new("rsync")
        .args([
            "-r",
            "--numeric-ids",
            "--owner",
            "--group",
            &src_arg,
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .expect("rsync not installed");

    let our_meta = fs::metadata(ours.join("id.txt")).unwrap();
    let rsync_meta = fs::metadata(rsync_dst.join("id.txt")).unwrap();
    assert_eq!(our_meta.uid(), rsync_meta.uid());
    assert_eq!(our_meta.gid(), rsync_meta.gid());
}
