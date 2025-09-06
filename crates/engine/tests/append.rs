// crates/engine/tests/append.rs
use compress::available_codecs;
use engine::{EngineError, SyncOptions, sync};
use filters::Matcher;
use std::fs;
use tempfile::tempdir;

#[test]
fn append_errors_when_destination_missing() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"data").unwrap();

    let opts = SyncOptions {
        append: true,
        ..Default::default()
    };
    let err = sync(&src, &dst, &Matcher::default(), &available_codecs(), &opts)
        .expect_err("expected error when appending without destination");
    if let EngineError::Io(e) = err {
        assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
    } else {
        panic!("unexpected error type: {err:?}");
    }
}
