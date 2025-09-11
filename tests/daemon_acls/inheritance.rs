// tests/daemon_acls/inheritance.rs
use super::*;

#[test]
#[serial]
fn daemon_inherits_directory_default_acls() {
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

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &src);

    let sub = src.join("sub");
    fs::create_dir(&sub).unwrap();

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let src_dacl = read_default_acl_or_skip!(&src);
    let src_sub_dacl = read_default_acl_or_skip!(&sub);
    let dst_dacl = read_default_acl_or_skip!(&srv);
    let dst_sub_dacl = read_default_acl_or_skip!(srv.join("sub"));
    assert_eq!(
        (src_dacl.entries(), src_sub_dacl.entries()),
        (dst_dacl.entries(), dst_sub_dacl.entries())
    );
}
#[test]
#[serial]
fn daemon_inherits_default_acls_rr_client() {
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
    write_default_acl_or_skip!(dacl, &src);

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

    let dacl_src = read_default_acl_or_skip!(&src);
    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert_eq!(dacl_src.entries(), dacl_dst.entries());

    let dacl_src_sub = read_default_acl_or_skip!(&sub);
    let dacl_dst_sub = read_default_acl_or_skip!(srv.join("sub"));
    assert_eq!(dacl_src_sub.entries(), dacl_dst_sub.entries());

    let acl_src_file = read_acl_or_skip!(&src_file);
    let acl_dst_file = read_acl_or_skip!(srv.join("sub/file"));
    assert_eq!(acl_src_file.entries(), acl_dst_file.entries());
}
