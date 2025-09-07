// tests/filter_errors.rs
use assert_cmd::Command;
use protocol::ExitCode;
use std::fs;
use tempfile::tempdir;

#[test]
fn invalid_rsync_filter_file_aborts() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.txt"), "data").unwrap();
    fs::write(src.join(".rsync-filter"), "[\n").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--recursive")
        .args(["--filter", "dir-merge .rsync-filter"])
        .arg(&src_arg)
        .arg(&dst)
        .assert()
        .failure()
        .code(u8::from(ExitCode::Protocol) as i32);

    assert!(!dst.join("file.txt").exists());
}

#[test]
fn invalid_filter_arg_aborts() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.txt"), "data").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--recursive")
        .args(["--filter", "bogus"])
        .arg(&src_arg)
        .arg(&dst)
        .assert()
        .failure()
        .code(u8::from(ExitCode::Protocol) as i32);
}
