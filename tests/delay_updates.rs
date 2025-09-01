// tests/delay_updates.rs
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;

use checksums::ChecksumConfigBuilder;
use engine::{compute_delta, Receiver, Result as EngineResult, SyncOptions};
use tempfile::tempdir;

#[test]
fn delay_updates_defers_rename() {
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    let dst_dir = tmp.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();

    let src_file = src_dir.join("file");
    let dst_file = dst_dir.join("file");
    fs::write(&src_file, b"new").unwrap();
    fs::write(&dst_file, b"old").unwrap();

    let opts = SyncOptions {
        delay_updates: true,
        ..Default::default()
    };
    let cfg = ChecksumConfigBuilder::new().build();
    let mut basis = BufReader::new(File::open(&dst_file).unwrap());
    let mut target = BufReader::new(File::open(&src_file).unwrap());
    let delta: Vec<_> = compute_delta(&cfg, &mut basis, &mut target, 4, 8 * 1024, &opts)
        .unwrap()
        .collect::<EngineResult<_>>()
        .unwrap();

    let rel = Path::new("file");
    let mut recv = Receiver::new(None, opts.clone());
    let tmp_path = recv
        .apply(&src_file, &dst_file, rel, delta.into_iter().map(Ok))
        .unwrap();
    assert_eq!(fs::read(&dst_file).unwrap(), b"old");
    assert_eq!(fs::read(&tmp_path).unwrap(), b"new");

    recv.copy_metadata(&src_file, &dst_file).unwrap();
    recv.finalize().unwrap();

    assert_eq!(fs::read(&dst_file).unwrap(), b"new");
    assert!(!tmp_path.exists());
}
