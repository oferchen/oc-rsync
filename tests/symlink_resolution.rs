// tests/symlink_resolution.rs

use assert_cmd::Command;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
#[cfg(unix)]
use tempfile::tempdir;

#[cfg(unix)]
fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) {
    std::os::unix::fs::symlink(src, dst).unwrap();
}

#[cfg(unix)]
fn assert_same_tree(a: &Path, b: &Path) {
    fn walk(a: &Path, b: &Path) {
        let mut ents_a: Vec<_> = fs::read_dir(a)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        ents_a.sort();
        let mut ents_b: Vec<_> = fs::read_dir(b)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        ents_b.sort();
        assert_eq!(ents_a, ents_b, "directory entries differ");
        for name in ents_a {
            let pa = a.join(&name);
            let pb = b.join(&name);
            let ma = fs::symlink_metadata(&pa).unwrap();
            let mb = fs::symlink_metadata(&pb).unwrap();
            assert_eq!(
                ma.file_type(),
                mb.file_type(),
                "file type differs for {:?}",
                name
            );
            assert_eq!(
                ma.permissions().mode(),
                mb.permissions().mode(),
                "permissions differ for {:?}",
                name
            );
            if ma.file_type().is_file() {
                assert_eq!(
                    fs::read(&pa).unwrap(),
                    fs::read(&pb).unwrap(),
                    "file contents differ for {:?}",
                    name
                );
            } else if ma.file_type().is_dir() {
                walk(&pa, &pb);
            } else if ma.file_type().is_symlink() {
                assert_eq!(
                    fs::read_link(&pa).unwrap(),
                    fs::read_link(&pb).unwrap(),
                    "symlink target differs for {:?}",
                    name
                );
            }
        }
    }
    walk(a, b);
}

#[cfg(unix)]
#[test]
fn copy_links_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst_rr = tmp.path().join("dst_rr");
    fs::create_dir_all(src.join("dir")).unwrap();
    fs::write(src.join("dir/file"), b"hi").unwrap();
    symlink("dir", src.join("dirlink"));

    let ours_out = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--copy-links",
            &format!("{}/", src.display()),
            dst_rr.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(ours_out.status.success());
    let ours_output = String::from_utf8_lossy(&ours_out.stdout).to_string()
        + &String::from_utf8_lossy(&ours_out.stderr);
    assert!(ours_output.is_empty());
    assert_same_tree(
        Path::new("tests/golden/symlink_resolution/copy_links"),
        &dst_rr,
    );
}

#[cfg(unix)]
#[test]
fn copy_unsafe_links_matches_rsync() {
    let tmp = tempdir().unwrap();
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("file"), b"hi").unwrap();
    let src = tmp.path().join("src");
    let dst_rr = tmp.path().join("dst_rr");
    fs::create_dir_all(&src).unwrap();
    symlink("../outside/file", src.join("link"));

    let ours_out = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--copy-unsafe-links",
            &format!("{}/", src.display()),
            dst_rr.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(ours_out.status.success());
    let ours_output = String::from_utf8_lossy(&ours_out.stdout).to_string()
        + &String::from_utf8_lossy(&ours_out.stderr);
    assert!(ours_output.is_empty());
    assert_same_tree(
        Path::new("tests/golden/symlink_resolution/copy_unsafe_links"),
        &dst_rr,
    );
}

#[cfg(unix)]
#[test]
fn safe_links_matches_rsync() {
    let tmp = tempdir().unwrap();
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("file"), b"hi").unwrap();
    let src = tmp.path().join("src");
    let dst_rr = tmp.path().join("dst_rr");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("inside"), b"ok").unwrap();
    symlink("inside", src.join("safe"));
    symlink("../outside/file", src.join("unsafe"));

    let ours_out = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--links",
            "--safe-links",
            &format!("{}/", src.display()),
            dst_rr.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(ours_out.status.success());
    let ours_output = String::from_utf8_lossy(&ours_out.stdout).to_string()
        + &String::from_utf8_lossy(&ours_out.stderr);
    assert!(ours_output.is_empty());
    assert_same_tree(
        Path::new("tests/golden/symlink_resolution/safe_links"),
        &dst_rr,
    );
}

#[cfg(unix)]
#[test]
fn munge_links_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst_rr = tmp.path().join("dst_rr");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    symlink("file", src.join("link"));

    let ours_out = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--links",
            "--munge-links",
            &format!("{}/", src.display()),
            dst_rr.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(ours_out.status.success());
    let ours_output = String::from_utf8_lossy(&ours_out.stdout).to_string()
        + &String::from_utf8_lossy(&ours_out.stderr);
    assert!(ours_output.is_empty());
    assert_same_tree(
        Path::new("tests/golden/symlink_resolution/munge_links"),
        &dst_rr,
    );
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
