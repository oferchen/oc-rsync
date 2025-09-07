// tests/rsync_filter_path_anchor.rs
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn exclude_star_blocks_nested_files() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("dir/sub")).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(src.join("root.txt"), "root").unwrap();
    fs::write(src.join("dir/file.txt"), "dir").unwrap();
    fs::write(src.join("dir/sub/nested.txt"), "nested").unwrap();

    let src_arg = format!("{}/", src.display());
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.arg("--recursive")
        .args(["--exclude", "*"])
        .arg(&src_arg)
        .arg(&dst);
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "oc-rsync failed: {:?}", out);

    assert!(!dst.join("root.txt").exists());
    assert!(!dst.join("dir/file.txt").exists());
    assert!(!dst.join("dir/sub/nested.txt").exists());
}

#[test]
fn rooted_pattern_applies_from_transfer_root() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("sub/nested")).unwrap();
    fs::create_dir_all(src.join("deep/sub")).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(src.join("sub/a.txt"), "a").unwrap();
    fs::write(src.join("sub/nested/b.txt"), "b").unwrap();
    fs::write(src.join("deep/sub/c.txt"), "c").unwrap();

    let src_arg = format!("{}/", src.display());
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.arg("--recursive")
        .args(["--exclude", "/sub/*"])
        .arg(&src_arg)
        .arg(&dst);
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "oc-rsync failed: {:?}", out);

    assert!(!dst.join("sub/a.txt").exists());
    assert!(!dst.join("sub/nested/b.txt").exists());
    assert!(dst.join("deep/sub/c.txt").exists());
}
