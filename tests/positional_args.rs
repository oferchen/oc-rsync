// tests/positional_args.rs
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn files_from_and_exclude_accept_src_and_dst() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("keep.txt"), "hi").unwrap();
    let list = tmp.path().join("list.txt");
    fs::write(&list, "keep.txt\n").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--dry-run",
            "--recursive",
            "--files-from",
            list.to_str().unwrap(),
            "--exclude",
            "*.log",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn exclude_then_files_from_accepts_src_and_dst() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("keep.txt"), "hi").unwrap();
    let list = tmp.path().join("list.txt");
    fs::write(&list, "keep.txt\n").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--dry-run",
            "--recursive",
            "--exclude",
            "*.log",
            "--files-from",
            list.to_str().unwrap(),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
}
