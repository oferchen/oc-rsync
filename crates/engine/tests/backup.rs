use std::fs;

use compress::available_codecs;
use engine::{sync, DeleteMode, SyncOptions};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn backups_replaced_and_deleted_files() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    let backup = tmp.path().join("backup");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(dst.join("file.txt"), b"old").unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    fs::write(src.join("file.txt"), b"new").unwrap();
    fs::write(dst.join("old.txt"), b"obsolete").unwrap();

    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions {
            delete: Some(DeleteMode::During),
            backup: true,
            backup_dir: Some(backup.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"new");
    assert_eq!(fs::read(backup.join("file.txt")).unwrap(), b"old");
    assert_eq!(fs::read(backup.join("old.txt")).unwrap(), b"obsolete");
    assert!(!dst.join("old.txt").exists());
}
