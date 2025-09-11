// tests/daemon_acls/removal.rs
use super::*;

#[test]
#[serial]
fn daemon_removes_file_acls() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let _src_file = src.join("file");
    fs::write(&_src_file, b"hi").unwrap();
    let srv_file = srv.join("file");
    fs::write(&srv_file, b"hi").unwrap();

    let mut acl = read_acl_or_skip!(&srv_file);
    acl.set(Qualifier::User(12345), ACL_READ);
    write_acl_or_skip!(acl, &srv_file);

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_dst = read_acl_or_skip!(&srv_file);
    assert!(acl_dst.get(Qualifier::User(12345)).is_none());
}
#[test]
#[serial]
fn daemon_removes_default_acls() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let _src_file = src.join("file");
    fs::write(&_src_file, b"hi").unwrap();

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &srv);

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert!(dacl_dst.entries().is_empty());
}
#[test]
#[serial]
fn daemon_ignores_acls_without_flag() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let (_tmp, src, srv) = setup_acl_dirs(|src| {
        let src_file = src.join("file");
        fs::write(&src_file, b"hi").unwrap();
        let mut acl = read_acl_or_skip!(&src_file);
        acl.set(Qualifier::User(12345), ACL_READ);
        write_acl_or_skip!(acl, &src_file);
    });

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([&src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_dst = read_acl_or_skip!(srv.join("file"));
    assert!(acl_dst.get(Qualifier::User(12345)).is_none());
}
#[test]
#[serial]
fn daemon_ignores_default_acls_without_flag() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let (_tmp, src, srv) = setup_acl_dirs(|src| {
        let mut dacl = PosixACL::new(0o755);
        dacl.set(Qualifier::User(12345), ACL_READ);
        write_default_acl_or_skip!(dacl, src);

        let sub = src.join("sub");
        fs::create_dir(&sub).unwrap();
        let src_file = sub.join("file");
        fs::write(&src_file, b"hi").unwrap();
    });

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([&src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert!(dacl_dst.get(Qualifier::User(12345)).is_none());

    let dacl_dst_sub = read_default_acl_or_skip!(srv.join("sub"));
    assert!(dacl_dst_sub.get(Qualifier::User(12345)).is_none());

    let acl_dst_file = read_acl_or_skip!(srv.join("sub/file"));
    assert!(acl_dst_file.get(Qualifier::User(12345)).is_none());
}
#[test]
#[serial]
fn daemon_removes_default_acls_rr_client() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &srv);

    let sub = src.join("sub");
    fs::create_dir(&sub).unwrap();
    let src_file = sub.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert!(dacl_dst.entries().is_empty());

    let dacl_dst_sub = read_default_acl_or_skip!(srv.join("sub"));
    assert!(dacl_dst_sub.entries().is_empty());

    let acl_src_file = read_acl_or_skip!(&src_file);
    let acl_dst_file = read_acl_or_skip!(srv.join("sub/file"));
    assert_eq!(acl_src_file.entries(), acl_dst_file.entries());
}
