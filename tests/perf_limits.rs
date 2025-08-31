// tests/perf_limits.rs

use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn max_alloc_limits_large_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("large.bin"), vec![0u8; 2048]).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--max-alloc=1024",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .failure();
}

#[test]
fn max_alloc_zero_is_unlimited() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("large.bin"), vec![0u8; 2048]).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--max-alloc=0",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn preallocate_option_creates_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.bin"), vec![0u8; 1024]).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--preallocate",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(fs::metadata(dst.join("file.bin")).unwrap().len(), 1024);
}

#[test]
fn max_size_skips_large_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("small.bin"), vec![0u8; 2048]).unwrap();
    fs::write(src.join("large.bin"), vec![0u8; 4096]).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--max-size=3k",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("small.bin").exists());
    assert!(!dst.join("large.bin").exists());
}

#[test]
fn min_size_skips_small_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("small.bin"), vec![0u8; 2048]).unwrap();
    fs::write(src.join("large.bin"), vec![0u8; 4096]).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--recursive",
            "--min-size=3k",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(!dst.join("small.bin").exists());
    assert!(dst.join("large.bin").exists());
}
