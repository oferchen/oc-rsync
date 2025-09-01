// crates/engine/tests/resume.rs
use checksums::ChecksumConfigBuilder;
use compress::available_codecs;
use engine::{compute_delta, sync, Op, SyncOptions};
use filters::Matcher;
use std::fs::{self, File};
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

    let mut partial_data = Vec::new();
    partial_data.extend_from_slice(&block);
    partial_data.extend_from_slice(&vec![0u8; 1024]);
    fs::write(dst.join("file.bin.partial"), &partial_data).unwrap();

    let mut opts = SyncOptions::default();
    opts.partial = true;
    sync(&src, &dst, &Matcher::default(), &available_codecs(), &opts).unwrap();

    let out = fs::read(dst.join("file.bin")).unwrap();
    assert_eq!(out, src_data);
}

#[test]
fn resume_large_file_minimal_network_io() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let block_size = 1024;
    let total_blocks = 256;
    let total_len = block_size * total_blocks;
    let partial_len = total_len / 2;
    let mut data = Vec::with_capacity(total_len);
    data.extend(std::iter::repeat(1u8).take(partial_len));
    data.extend(std::iter::repeat(2u8).take(total_len - partial_len));
    fs::write(src.join("big.bin"), &data).unwrap();

    fs::write(dst.join("big.bin.partial"), &data[..partial_len]).unwrap();

    let cfg = ChecksumConfigBuilder::new().build();
    let mut basis = File::open(dst.join("big.bin.partial")).unwrap();
    let mut target = File::open(src.join("big.bin")).unwrap();
    let delta: Vec<Op> = compute_delta(
        &cfg,
        &mut basis,
        &mut target,
        block_size,
        usize::MAX,
        &SyncOptions::default(),
    )
    .unwrap()
    .collect::<engine::Result<_>>()
    .unwrap();
    let mut sent = 0usize;
    for op in &delta {
        if let Op::Data(d) = op {
            sent += d.len();
        }
    }
    assert_eq!(sent, data.len() - partial_len);

    let mut opts = SyncOptions::default();
    opts.partial = true;
    opts.block_size = block_size;
    sync(&src, &dst, &Matcher::default(), &available_codecs(), &opts).unwrap();
    assert_eq!(fs::read(dst.join("big.bin")).unwrap(), data);
}
