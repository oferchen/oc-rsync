// tests/out_format.rs
use assert_cmd::Command as TestCommand;
use std::fs;
use tempfile::tempdir;

#[test]
fn out_format_file_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("a"), b"hi").unwrap();
    let dst_oc = tmp.path().join("dst_oc");
    fs::create_dir_all(&dst_oc).unwrap();
    let log = tmp.path().join("log.txt");
    let src_arg = format!("{}/", src_dir.display());

    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--log-file",
            log.to_str().unwrap(),
            "--out-format=%o:%n",
            &src_arg,
            dst_oc.to_str().unwrap(),
        ])
        .assert()
        .success();
    let ours = fs::read_to_string(&log).unwrap();
    let ours_msg = ours.lines().find(|l| l.trim() == "send:a").unwrap().trim();

    let binding =
        fs::read_to_string("tests/golden/out_format/out_format_file_matches_rsync.stdout").unwrap();
    let theirs_msg = binding
        .lines()
        .find(|l| l.trim() == "send:a")
        .unwrap()
        .trim();

    assert_eq!(ours_msg, theirs_msg);
}

#[cfg(unix)]
#[test]
fn out_format_symlink_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("f"), b"hi").unwrap();
    std::os::unix::fs::symlink("f", src_dir.join("link")).unwrap();
    let dst_oc = tmp.path().join("dst_oc");
    fs::create_dir_all(&dst_oc).unwrap();
    let log = tmp.path().join("log.txt");
    let src_arg = format!("{}/", src_dir.display());

    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-l",
            "--recursive",
            "--log-file",
            log.to_str().unwrap(),
            "--out-format=%i:%n%L",
            &src_arg,
            dst_oc.to_str().unwrap(),
        ])
        .assert()
        .success();
    let ours = fs::read_to_string(&log).unwrap();
    let ours_msg = ours.lines().find(|l| l.contains("link")).unwrap().trim();

    let binding =
        fs::read_to_string("tests/golden/out_format/out_format_symlink_matches_rsync.stdout")
            .unwrap();
    let theirs_msg = binding.lines().find(|l| l.contains("link")).unwrap().trim();

    assert_eq!(ours_msg, theirs_msg);
}
