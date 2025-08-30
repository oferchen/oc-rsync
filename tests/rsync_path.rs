// tests/rsync_path.rs
use assert_cmd::Command;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

#[cfg(unix)]
#[test]
fn custom_rsync_path_performs_transfer() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    let src_file = src_dir.join("file.txt");
    fs::write(&src_file, b"from custom binary").unwrap();
    let dst_dir = dir.path().join("dst");

    let remote_bin = dir.path().join("rr-remote");
    fs::copy(assert_cmd::cargo::cargo_bin("oc-rsync"), &remote_bin).unwrap();
    let mut perms = fs::metadata(&remote_bin).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&remote_bin, perms).unwrap();

    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = format!("ignored:{}", dst_dir.display());
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args([
        "-e",
        rsh.to_str().unwrap(),
        "--rsync-path",
        remote_bin.to_str().unwrap(),
        "-r",
        &src_spec,
        &dst_spec,
    ]);
    cmd.assert().success();

    let out = fs::read(dst_dir.join("file.txt")).unwrap();
    assert_eq!(out, b"from custom binary");
}

#[cfg(unix)]
#[test]
fn rsync_path_supports_wrapper_command() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    let src_file = src_dir.join("file.txt");
    fs::write(&src_file, b"wrapped").unwrap();
    let dst_dir = dir.path().join("dst");

    let remote_bin = dir.path().join("rr-remote");
    fs::copy(assert_cmd::cargo::cargo_bin("oc-rsync"), &remote_bin).unwrap();
    fs::set_permissions(&remote_bin, fs::Permissions::from_mode(0o755)).unwrap();

    let marker = dir.path().join("marker.txt");
    let wrapper = dir.path().join("wrapper.sh");
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\ntouch {}\nexec {} \"$@\"\n",
            marker.display(),
            remote_bin.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();

    let rsync_path = format!("{} {}", wrapper.display(), remote_bin.display());

    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = format!("ignored:{}", dst_dir.display());
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args([
        "-e",
        rsh.to_str().unwrap(),
        "--rsync-path",
        &rsync_path,
        "-r",
        &src_spec,
        &dst_spec,
    ]);
    cmd.assert().success();

    assert!(marker.exists());
    let out = fs::read(dst_dir.join("file.txt")).unwrap();
    assert_eq!(out, b"wrapped");
}
