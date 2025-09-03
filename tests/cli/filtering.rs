// tests/cli/filtering.rs

use assert_cmd::prelude::*;
use assert_cmd::Command;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[test]
fn files_from_from0_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("include_me.txt"), "hi").unwrap();
    fs::write(src.join("omit.log"), "nope").unwrap();

    let list = tmp.path().join("list");
    fs::write(&list, b"include_me.txt\0omit.log\0").unwrap();

    let src_arg = format!("{}/", src.display());

    StdCommand::new("rsync")
        .args([
            "-r",
            "--from0",
            "--files-from",
            list.to_str().unwrap(),
            "--exclude",
            "*.log",
            &src_arg,
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--from0",
            "--files-from",
            list.to_str().unwrap(),
            "--exclude",
            "*.log",
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn include_from_from0_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("a.txt"), "hi").unwrap();
    fs::write(src.join("b.log"), "nope").unwrap();
    fs::write(src.join("c.txt"), "hi").unwrap();

    let list = tmp.path().join("list");
    fs::write(&list, b"a.txt\0c.txt\0").unwrap();

    let src_arg = format!("{}/", src.display());

    StdCommand::new("rsync")
        .args([
            "-r",
            "--from0",
            "--include-from",
            list.to_str().unwrap(),
            "--exclude",
            "*",
            &src_arg,
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--from0",
            "--include-from",
            list.to_str().unwrap(),
            "--exclude",
            "*",
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn exclude_from_from0_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("keep.txt"), "hi").unwrap();
    fs::write(src.join("drop.txt"), "nope").unwrap();

    let list = tmp.path().join("list");
    fs::write(&list, b"drop.txt\0").unwrap();

    let src_arg = format!("{}/", src.display());

    StdCommand::new("rsync")
        .args([
            "-r",
            "--from0",
            "--exclude-from",
            list.to_str().unwrap(),
            &src_arg,
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--from0",
            "--exclude-from",
            list.to_str().unwrap(),
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn filter_file_from0_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("a.txt"), "hi").unwrap();
    fs::write(src.join("b.log"), "no").unwrap();
    fs::write(src.join("c.txt"), "hi").unwrap();

    let filter = tmp.path().join("filters");
    fs::write(&filter, b"+ *.txt\0- *\0").unwrap();

    let src_arg = format!("{}/", src.display());

    StdCommand::new("rsync")
        .args([
            "-r",
            "--from0",
            "--filter",
            &format!("merge {}", filter.display()),
            &src_arg,
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--from0",
            "--filter-file",
            filter.to_str().unwrap(),
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn per_dir_merge_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("keep.txt"), "hi").unwrap();
    fs::write(src.join("omit.log"), "no").unwrap();
    fs::write(src.join("sub").join("keep2.txt"), "hi").unwrap();
    fs::write(src.join("sub").join("omit2.txt"), "no").unwrap();

    fs::write(src.join(".rsync-filter"), b"- *.log\n").unwrap();
    fs::write(src.join("sub").join(".rsync-filter"), b"- omit2.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());

    StdCommand::new("rsync")
        .args(["-r", "-F", "-F", &src_arg, rsync_dst.to_str().unwrap()])
        .status()
        .unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "-F",
            "-F",
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}
#[test]
fn exclude_from_from0_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("keep.txt"), "hi").unwrap();
    fs::write(src.join("drop.txt"), "nope").unwrap();

    let list = tmp.path().join("list");
    fs::write(&list, b"drop.txt\0").unwrap();

    let src_arg = format!("{}/", src.display());

    StdCommand::new("rsync")
        .args([
            "-r",
            "--from0",
            "--exclude-from",
            list.to_str().unwrap(),
            &src_arg,
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--from0",
            "--exclude-from",
            list.to_str().unwrap(),
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn filter_file_from0_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("a.txt"), "hi").unwrap();
    fs::write(src.join("b.log"), "no").unwrap();
    fs::write(src.join("c.txt"), "hi").unwrap();

    let filter = tmp.path().join("filters");
    fs::write(&filter, b"+ *.txt\0- *\0").unwrap();

    let src_arg = format!("{}/", src.display());

    StdCommand::new("rsync")
        .args([
            "-r",
            "--from0",
            "--filter",
            &format!("merge {}", filter.display()),
            &src_arg,
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--from0",
            "--filter-file",
            filter.to_str().unwrap(),
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}

#[test]
fn per_dir_merge_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::write(src.join("keep.txt"), "hi").unwrap();
    fs::write(src.join("omit.log"), "no").unwrap();
    fs::write(src.join("sub").join("keep2.txt"), "hi").unwrap();
    fs::write(src.join("sub").join("omit2.txt"), "no").unwrap();

    fs::write(src.join(".rsync-filter"), b"- *.log\n").unwrap();
    fs::write(src.join("sub").join(".rsync-filter"), b"- omit2.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());

    StdCommand::new("rsync")
        .args(["-r", "-F", "-F", &src_arg, rsync_dst.to_str().unwrap()])
        .status()
        .unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "-F",
            "-F",
            &src_arg,
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}
