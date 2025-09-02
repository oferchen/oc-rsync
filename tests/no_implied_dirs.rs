// tests/no_implied_dirs.rs
use assert_cmd::Command;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use tempfile::tempdir;

#[test]
#[ignore]
fn preserves_symlinked_implied_dirs() {
    let tmp = tempdir().unwrap();
    let src_root = tmp.path().join("src");
    fs::create_dir_all(src_root.join("path/foo")).unwrap();
    fs::write(src_root.join("path/foo/file"), b"data").unwrap();

    let rsync_dst = tmp.path().join("rsync_dst");
    let oc_dst = tmp.path().join("oc_dst");
    fs::create_dir_all(rsync_dst.join("path")).unwrap();
    fs::create_dir_all(oc_dst.join("path")).unwrap();
    symlink("bar", rsync_dst.join("path/foo")).unwrap();
    symlink("bar", oc_dst.join("path/foo")).unwrap();
    fs::create_dir_all(rsync_dst.join("path/bar")).unwrap();
    fs::create_dir_all(oc_dst.join("path/bar")).unwrap();

    let rel_path = "path/foo/file";
    let rsync_dest = format!("{}/", rsync_dst.display());
    Command::new("rsync")
        .current_dir(&src_root)
        .args(["-R", "--no-implied-dirs", rel_path, &rsync_dest])
        .assert()
        .success();

    let oc_dest = format!("{}/", oc_dst.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .current_dir(&src_root)
        .args([
            "--local",
            "--relative",
            "--no-implied-dirs",
            rel_path,
            &oc_dest,
        ])
        .assert()
        .success();

    let rs_meta = fs::symlink_metadata(rsync_dst.join("path/foo")).unwrap();
    let oc_meta = fs::symlink_metadata(oc_dst.join("path/foo")).unwrap();
    assert!(rs_meta.file_type().is_symlink());
    assert!(oc_meta.file_type().is_symlink());
    assert_eq!(
        fs::read_link(rsync_dst.join("path/foo")).unwrap(),
        fs::read_link(oc_dst.join("path/foo")).unwrap()
    );
    let rs_file = rsync_dst.join("path/bar/file");
    let oc_file = oc_dst.join("path/bar/file");
    assert!(rs_file.exists());
    assert!(oc_file.exists());
    assert_eq!(fs::read(rs_file).unwrap(), fs::read(oc_file).unwrap());
}
