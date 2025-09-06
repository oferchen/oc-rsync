// tests/rsync_filter_dir_ancestors.rs
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn include_pattern_traverses_ancestors() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("a/b")).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(src.join("a/b/keep.txt"), "hi").unwrap();
    fs::write(src.join("a/b/omit.txt"), "no").unwrap();
    fs::write(src.join(".rsync-filter"), b"+ /a/b/keep.txt\n- omit.txt\n").unwrap();

    let src_arg = format!("{}/", src.display());
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.arg("--recursive")
        .args(["-F", "-F"])
        .arg(&src_arg)
        .arg(&dst);
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "oc-rsync failed: {:?}", out);

    assert!(dst.join("a/b/keep.txt").exists());
    assert!(!dst.join("a/b/omit.txt").exists());
}
