// tests/include_from.rs
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn include_from_creates_parents_and_excludes_siblings() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("a/b")).unwrap();
    fs::write(src.join("a/b/keep.txt"), b"data").unwrap();
    fs::write(src.join("a/b/skip.txt"), b"no").unwrap();
    fs::write(src.join("a/other.txt"), b"no").unwrap();
    fs::write(src.join("top.txt"), b"no").unwrap();
    fs::create_dir_all(&dst).unwrap();

    let list = tmp.path().join("list");
    fs::write(&list, "a/b/keep.txt\n").unwrap();
    let src_arg = format!("{}/", src.display());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--include-from",
            list.to_str().unwrap(),
            "--exclude",
            "*",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("a").is_dir());
    assert!(dst.join("a/b").is_dir());
    assert!(dst.join("a/b/keep.txt").is_file());
    assert!(!dst.join("a/b/skip.txt").exists());
    assert!(!dst.join("a/other.txt").exists());
    assert!(!dst.join("top.txt").exists());
}
