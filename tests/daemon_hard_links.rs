// tests/daemon_hard_links.rs
use assert_cmd::Command;
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

mod common;
use common::daemon::{spawn_daemon, wait_for_daemon};

#[cfg(unix)]
#[test]
#[serial]
fn daemon_preserves_hard_links_multiple() {
    let tmp = tempdir().expect("create temp dir");
    let src = tmp.path().join("src");
    let module = tmp.path().join("mod");
    fs::create_dir_all(&src).expect("create source dir");
    fs::create_dir_all(&module).expect("create module dir");

    let f1 = src.join("a");
    fs::write(&f1, b"hi").expect("write a");
    let f2 = src.join("b");
    fs::hard_link(&f1, &f2).expect("link b to a");
    let f3 = src.join("c");
    fs::hard_link(&f1, &f3).expect("link c to a");

    let mut daemon = spawn_daemon(&module);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    let dest = format!("rsync://127.0.0.1:{port}/mod/");
    Command::cargo_bin("oc-rsync")
        .expect("oc-rsync binary")
        .current_dir(&tmp)
        .args(["-aH", &src_arg, &dest])
        .assert()
        .success();

    assert!(!tmp.path().join("rsync:").exists());

    let ino_a = fs::metadata(module.join("a")).expect("stat a").ino();
    let ino_b = fs::metadata(module.join("b")).expect("stat b").ino();
    let ino_c = fs::metadata(module.join("c")).expect("stat c").ino();
    assert_eq!(ino_a, ino_b);
    assert_eq!(ino_a, ino_c);
}
