// tests/daemon_acls/preservation.rs
use super::*;

#[test]
#[serial]
fn daemon_preserves_file_acls() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut acl = read_acl_or_skip!(&src_file);
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    write_acl_or_skip!(acl, &src_file);

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_src = read_acl_or_skip!(&src_file);
    let acl_dst = read_acl_or_skip!(srv.join("file"));
    assert_eq!(acl_src.entries(), acl_dst.entries());
}

#[test]
#[serial]
fn daemon_preserves_default_acls() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &src);

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_src = read_default_acl_or_skip!(&src);
    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert_eq!(dacl_src.entries(), dacl_dst.entries());
}

#[test]
#[serial]
fn daemon_preserves_default_acls_with_flag() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &src);

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_src = read_default_acl_or_skip!(&src);
    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert_eq!(dacl_src.entries(), dacl_dst.entries());
}

#[test]
#[serial]
fn daemon_preserves_nested_default_acls() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();

    let sub = src.join("sub");
    fs::create_dir(&sub).unwrap();

    let mut root_dacl = PosixACL::new(0o755);
    root_dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(root_dacl, &src);

    let mut sub_dacl = PosixACL::new(0o700);
    sub_dacl.set(Qualifier::User(23456), ACL_READ);
    write_default_acl_or_skip!(sub_dacl, &sub);

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let src_root_dacl = read_default_acl_or_skip!(&src);
    let src_sub_dacl = read_default_acl_or_skip!(&sub);
    let dst_root_dacl = read_default_acl_or_skip!(&srv);
    let dst_sub_dacl = read_default_acl_or_skip!(srv.join("sub"));
    assert_eq!(
        (src_root_dacl.entries(), src_sub_dacl.entries()),
        (dst_root_dacl.entries(), dst_sub_dacl.entries())
    );
}

#[test]
#[serial]
fn daemon_preserves_acls_rr_client() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut acl = read_acl_or_skip!(&src_file);
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    write_acl_or_skip!(acl, &src_file);

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &src);

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_src = read_acl_or_skip!(&src_file);
    let acl_dst = read_acl_or_skip!(srv.join("file"));
    assert_eq!(acl_src.entries(), acl_dst.entries());

    let dacl_src = read_default_acl_or_skip!(&src);
    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert_eq!(dacl_src.entries(), dacl_dst.entries());
}

#[test]
#[serial]
fn daemon_preserves_root_sub_and_file_acls() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();

    let mut root_dacl = PosixACL::new(0o755);
    root_dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(root_dacl, &src);

    let sub = src.join("sub");
    fs::create_dir(&sub).unwrap();
    let mut sub_dacl = PosixACL::new(0o700);
    sub_dacl.set(Qualifier::User(23456), ACL_READ);
    write_default_acl_or_skip!(sub_dacl, &sub);

    let src_file = sub.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let src_root_dacl = read_default_acl_or_skip!(&src);
    let dst_root_dacl = read_default_acl_or_skip!(&srv);
    assert_eq!(src_root_dacl.entries(), dst_root_dacl.entries());

    let src_sub_dacl = read_default_acl_or_skip!(&sub);
    let dst_sub_dacl = read_default_acl_or_skip!(srv.join("sub"));
    assert_eq!(src_sub_dacl.entries(), dst_sub_dacl.entries());

    let acl_src_file = read_acl_or_skip!(&src_file);
    let acl_dst_file = read_acl_or_skip!(srv.join("sub/file"));
    assert_eq!(acl_src_file.entries(), acl_dst_file.entries());
}

#[test]
#[serial]
fn local_preserves_default_acls() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &src);

    let sub = src.join("sub");
    fs::create_dir(&sub).unwrap();
    let file = sub.join("file");
    fs::write(&file, b"hi").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let src_root_dacl = read_default_acl_or_skip!(&src);
    let dst_root_dacl = read_default_acl_or_skip!(&dst);
    assert_eq!(src_root_dacl.entries(), dst_root_dacl.entries());

    let src_sub_dacl = read_default_acl_or_skip!(&sub);
    let dst_sub_dacl = read_default_acl_or_skip!(dst.join("sub"));
    assert_eq!(src_sub_dacl.entries(), dst_sub_dacl.entries());

    let src_file_acl = read_acl_or_skip!(&file);
    let dst_file_acl = read_acl_or_skip!(dst.join("sub/file"));
    assert_eq!(src_file_acl.entries(), dst_file_acl.entries());
}
#[test]
#[serial]
fn daemon_preserves_nested_default_acls_rr_client() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();

    let sub = src.join("sub");
    fs::create_dir(&sub).unwrap();

    let mut root_dacl = PosixACL::new(0o755);
    root_dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(root_dacl, &src);

    let mut sub_dacl = PosixACL::new(0o700);
    sub_dacl.set(Qualifier::User(23456), ACL_READ);
    write_default_acl_or_skip!(sub_dacl, &sub);

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let src_root_dacl = read_default_acl_or_skip!(&src);
    let src_sub_dacl = read_default_acl_or_skip!(&sub);
    let dst_root_dacl = read_default_acl_or_skip!(&srv);
    let dst_sub_dacl = read_default_acl_or_skip!(srv.join("sub"));
    assert_eq!(
        (src_root_dacl.entries(), src_sub_dacl.entries()),
        (dst_root_dacl.entries(), dst_sub_dacl.entries())
    );
}

#[test]
#[serial]
fn daemon_serves_default_acls() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let sub = src.join("sub");
    fs::create_dir(&sub).unwrap();
    let file = sub.join("file");
    fs::write(&file, b"hi").unwrap();

    let mut root_dacl = PosixACL::new(0o755);
    root_dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(root_dacl, &src);

    let mut sub_dacl = PosixACL::new(0o700);
    sub_dacl.set(Qualifier::User(23456), ACL_READ);
    write_default_acl_or_skip!(sub_dacl, &sub);

    let mut daemon = spawn_daemon(&src);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_url = format!("rsync://127.0.0.1:{port}/mod/");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_url, dst.to_str().unwrap()])
        .assert()
        .success();

    let dst_root_dacl = read_default_acl_or_skip!(&dst);
    assert_eq!(root_dacl.entries(), dst_root_dacl.entries());

    let dst_sub_dacl = read_default_acl_or_skip!(dst.join("sub"));
    assert_eq!(sub_dacl.entries(), dst_sub_dacl.entries());

    let src_file_acl = read_acl_or_skip!(&file);
    let dst_file_acl = read_acl_or_skip!(dst.join("sub/file"));
    assert_eq!(src_file_acl.entries(), dst_file_acl.entries());
}
#[cfg(feature = "root")]
#[test]
#[serial]
fn daemon_file_acls_match_rsync_server() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let Some((_tmp, srv_oc, srv_rs)) = sync_daemon_acls_server() else {
        return;
    };
    let acl_oc = read_acl_or_skip!(srv_oc.join("file"));
    let acl_rs = read_acl_or_skip!(srv_rs.join("file"));
    assert_eq!(acl_oc.entries(), acl_rs.entries());
}
#[cfg(feature = "root")]
#[test]
#[serial]
fn daemon_default_acls_match_rsync_server() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let Some((_tmp, srv_oc, srv_rs)) = sync_daemon_acls_server() else {
        return;
    };
    let dacl_oc = read_default_acl_or_skip!(&srv_oc);
    let dacl_rs = read_default_acl_or_skip!(&srv_rs);
    assert_eq!(dacl_oc.entries(), dacl_rs.entries());
}
#[cfg(feature = "root")]
#[test]
#[serial]
fn daemon_file_acls_match_rsync_client() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let Some((_tmp, srv_oc, srv_rs)) = sync_daemon_acls_client() else {
        return;
    };
    let acl_oc = read_acl_or_skip!(srv_oc.join("file"));
    let acl_rs = read_acl_or_skip!(srv_rs.join("file"));
    assert_eq!(acl_oc.entries(), acl_rs.entries());
}
#[cfg(feature = "root")]
#[test]
#[serial]
fn daemon_default_acls_match_rsync_client() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let Some((_tmp, srv_oc, srv_rs)) = sync_daemon_acls_client() else {
        return;
    };
    let dacl_oc = read_default_acl_or_skip!(&srv_oc);
    let dacl_rs = read_default_acl_or_skip!(&srv_rs);
    assert_eq!(dacl_oc.entries(), dacl_rs.entries());
}
