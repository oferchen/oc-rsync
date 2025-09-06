// tests/rsync_filter_precedence.rs
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn rsync_filter_nested_precedence() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("logs/nested")).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(src.join(".rsync-filter"), "- *.tmp\n").unwrap();
    fs::write(src.join("logs/.rsync-filter"), "+ keep.tmp\n- *.tmp\n").unwrap();
    fs::write(src.join("logs/nested/.rsync-filter"), "- keep.tmp\n").unwrap();

    fs::write(src.join("logs/keep.tmp"), "keep").unwrap();
    fs::write(src.join("logs/other.tmp"), "other").unwrap();
    fs::write(src.join("logs/nested/keep.tmp"), "nested").unwrap();
    fs::write(src.join("logs/nested/other.tmp"), "nested-other").unwrap();
    fs::write(src.join("other.tmp"), "root").unwrap();

    let src_arg = format!("{}/", src.display());
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.arg("--recursive")
        .args(["--filter=: .rsync-filter", "--filter=- .rsync-filter"])
        .arg(&src_arg)
        .arg(&dst);
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "oc-rsync failed: {:?}", out);

    assert!(dst.join("logs/keep.tmp").exists());
    assert!(!dst.join("logs/other.tmp").exists());
    assert!(!dst.join("logs/nested/keep.tmp").exists());
    assert!(!dst.join("logs/nested/other.tmp").exists());
    assert!(!dst.join("other.tmp").exists());
}
