use compress::available_codecs;
use engine::{sync, SyncOptions};
use filters::Matcher;
use std::fs;
use tempfile::tempdir;

#[test]
fn resume_from_partial_file() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let block = vec![1u8; 1024];
    let block2 = vec![2u8; 1024];
    let block3 = vec![3u8; 1024];
    let mut src_data = Vec::new();
    src_data.extend_from_slice(&block);
    src_data.extend_from_slice(&block2);
    src_data.extend_from_slice(&block3);
    fs::write(src.join("file.bin"), &src_data).unwrap();

    // create partial file with first block correct, second block corrupted
    let mut partial_data = Vec::new();
    partial_data.extend_from_slice(&block);
    partial_data.extend_from_slice(&vec![0u8; 1024]);
    fs::write(dst.join("file.bin.partial"), &partial_data).unwrap();

    let mut opts = SyncOptions::default();
    opts.partial = true;
    sync(&src, &dst, &Matcher::default(), available_codecs(), &opts).unwrap();

    let out = fs::read(dst.join("file.bin")).unwrap();
    assert_eq!(out, src_data);
}
