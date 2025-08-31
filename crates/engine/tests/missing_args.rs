// crates/engine/tests/missing_args.rs
use std::fs;

use compress::available_codecs;
use engine::{sync, SyncOptions};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn deletes_missing_arg() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("missing.txt");
    let dst = tmp.path().join("dst.txt");
    fs::write(&dst, b"data").unwrap();
    let opts = SyncOptions {
        delete_missing_args: true,
        ..Default::default()
    };
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(None),
        &opts,
    )
    .unwrap();
    assert!(!dst.exists());
}
