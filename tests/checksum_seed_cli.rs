// tests/checksum_seed_cli.rs
use compress::available_codecs;
use engine::{sync, SyncOptions};
use filters::Matcher;
use std::fs;
use tempfile::tempdir;

#[test]
#[ignore]
fn checksum_seed_flag_transfers_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.txt"), b"seeded").unwrap();

    let opts = SyncOptions {
        checksum: true,
        checksum_seed: 1,
        ..Default::default()
    };
    // ensure destination exists as a directory
    fs::create_dir_all(&dst).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(None),
        &opts,
    )
    .unwrap();

    let out = fs::read(dst.join("a.txt")).unwrap();
    assert_eq!(out, b"seeded");
}
