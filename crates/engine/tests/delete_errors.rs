// crates/engine/tests/delete_errors.rs
#![doc = "Requires `CAP_CHOWN` to adjust directory ownership."]
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use compress::available_codecs;
use engine::{DeleteMode, SyncOptions, sync};
use filters::Matcher;
use nix::unistd::{Gid, Uid, chown};
mod tests;
use tempfile::tempdir;

#[test]
fn continues_deleting_after_io_errors() {
    if !tests::requires_capability(tests::CapabilityCheck::CapChown) {
        return;
    }
    if env::var("ENGINE_DELETE_CHILD").is_ok() {
        let src = PathBuf::from(env::var("SRC").unwrap());
        let dst = PathBuf::from(env::var("DST").unwrap());
        let res = sync(
            &src,
            &dst,
            &Matcher::default(),
            &available_codecs(),
            &SyncOptions {
                delete: Some(DeleteMode::During),
                ..Default::default()
            },
        );
        std::process::exit(if res.is_err() { 1 } else { 0 });
    }

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::set_permissions(&dst, fs::Permissions::from_mode(0o1777)).unwrap();
    fs::create_dir_all(dst.join("a")).unwrap();
    fs::create_dir_all(dst.join("b")).unwrap();
    chown(
        dst.join("b").as_path(),
        Some(Uid::from_raw(65534)),
        Some(Gid::from_raw(65534)),
    )
    .unwrap();

    let status = Command::new(env::current_exe().unwrap())
        .env("ENGINE_DELETE_CHILD", "1")
        .env("SRC", &src)
        .env("DST", &dst)
        .uid(65534)
        .gid(65534)
        .status()
        .unwrap();

    assert!(!status.success());
    assert!(dst.join("a").exists());
    assert!(!dst.join("b").exists());
}
