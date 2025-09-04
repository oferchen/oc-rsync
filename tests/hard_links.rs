// tests/hard_links.rs

use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(unix)]
#[test]
fn sync_preserves_link_counts() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let f1 = src.join("a");
    fs::write(&f1, b"hi").unwrap();
    let f2 = src.join("b");
    fs::hard_link(&f1, &f2).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--hard-links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let m1 = fs::metadata(dst.join("a")).unwrap();
    let m2 = fs::metadata(dst.join("b")).unwrap();
    assert_eq!(m1.ino(), m2.ino());
    assert_eq!(m1.nlink(), 2);
}
