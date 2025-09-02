// tests/symlink_resolution.rs

use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[cfg(unix)]
fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) {
    std::os::unix::fs::symlink(src, dst).unwrap();
}

#[cfg(unix)]
#[test]
fn copy_links_directory() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("dir")).unwrap();
    fs::write(src.join("dir/file"), b"hi").unwrap();
    symlink("dir", src.join("dirlink"));

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--copy-links",
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("dirlink").is_dir());
    assert_eq!(fs::read(dst.join("dirlink/file")).unwrap(), b"hi");
}

#[cfg(unix)]
#[test]
fn copy_unsafe_links() {
    let tmp = tempdir().unwrap();
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("file"), b"hi").unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    symlink("../outside/file", src.join("link"));

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--copy-unsafe-links",
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst.join("link").is_file());
    assert_eq!(fs::read(dst.join("link")).unwrap(), b"hi");
}

#[cfg(unix)]
#[test]
fn safe_links_skip() {
    let tmp = tempdir().unwrap();
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("file"), b"hi").unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("inside"), b"ok").unwrap();
    symlink("inside", src.join("safe"));
    symlink("../outside/file", src.join("unsafe"));

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--links",
            "--safe-links",
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst
        .join("safe")
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(!dst.join("unsafe").exists());
}

#[cfg(unix)]
#[test]
fn safe_links_resolve_source_symlink() {
    let tmp = tempdir().unwrap();
    let real = tmp.path().join("real");
    let link = tmp.path().join("link");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&real).unwrap();
    fs::write(real.join("file"), b"hi").unwrap();
    symlink("file", real.join("safe"));
    symlink("real", &link);

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--links",
            "--safe-links",
            &format!("{}/", link.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(dst
        .join("safe")
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());
}

#[cfg(unix)]
#[test]
fn munge_links_prefixes_target() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    symlink("file", src.join("link"));

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--links",
            "--munge-links",
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_link(dst.join("link")).unwrap(),
        Path::new("/rsyncd-munged/file")
    );
}

#[cfg(unix)]
#[test]
fn munge_links_off_preserves_target() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    symlink("file", src.join("link"));

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--links",
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(fs::read_link(dst.join("link")).unwrap(), Path::new("file"));
}

#[cfg(unix)]
#[test]
fn munge_links_unmunges_existing_prefix() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    symlink("/rsyncd-munged/file", src.join("link"));

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--links",
            "--munge-links",
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_link(dst.join("link")).unwrap(),
        Path::new("/rsyncd-munged/file")
    );
}
