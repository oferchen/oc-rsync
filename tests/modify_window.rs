// tests/modify_window.rs

use assert_cmd::Command;
use filetime::{set_file_mtime, FileTime};
use std::fs;
use tempfile::tempdir;

#[test]
fn modify_window_treats_close_times_as_equal() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    fs::write(dst.join("file"), b"hi").unwrap();

    let newer = FileTime::from_unix_time(1_000_000, 0);
    let older = FileTime::from_unix_time(999_999, 0);
    set_file_mtime(src.join("file"), newer).unwrap();
    set_file_mtime(dst.join("file"), older).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--itemize-changes",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(">f"));

    set_file_mtime(dst.join("file"), older).unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--itemize-changes",
            "--modify-window=2",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");
}
