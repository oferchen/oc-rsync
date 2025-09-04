// tests/interop/dry_run.rs

use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn dry_run_preserves_destination() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(src.join("new.txt"), b"new").unwrap();
    fs::write(dst.join("old.txt"), b"old").unwrap();

    let mut before: Vec<_> = fs::read_dir(&dst)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    before.sort();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--recursive", "--dry-run", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let mut after: Vec<_> = fs::read_dir(&dst)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    after.sort();

    assert_eq!(before, after);
    assert!(!dst.join("new.txt").exists());
    assert_eq!(fs::read_to_string(dst.join("old.txt")).unwrap(), "old");
}

