// tests/cdc.rs
#![cfg(feature = "blake3")]

use compress::available_codecs;
use engine::{sync, ModernCdc, SyncOptions};
use filters::Matcher;
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

#[test]
#[serial]
fn cdc_skips_renamed_file() {
    let home = tempdir().unwrap();
    std::env::set_var("HOME", home.path());

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let file_a = src.join("a.txt");
    fs::write(&file_a, vec![0u8; 4096]).unwrap();

    let opts = SyncOptions {
        modern_cdc: ModernCdc::Fastcdc,
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

    let file_b = src.join("b.txt");
    fs::rename(&file_a, &file_b).unwrap();

    let stats = sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(None),
        &opts,
    )
    .unwrap();
    assert_eq!(stats.bytes_transferred, 0);
    assert_eq!(fs::read(dst.join("b.txt")).unwrap(), vec![0u8; 4096]);
}

#[test]
#[serial]
fn cdc_reuses_manifest_with_custom_sizes() {
    let home = tempdir().unwrap();
    std::env::set_var("HOME", home.path());

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let file_a = src.join("a.bin");
    fs::write(&file_a, vec![0u8; 50 * 1024]).unwrap();

    let opts = SyncOptions {
        modern_cdc: ModernCdc::Fastcdc,
        modern_cdc_min: 4096,
        modern_cdc_max: 8192,
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

    assert!(home.path().join(".oc-rsync/manifest").exists());

    let file_b = src.join("b.bin");
    fs::rename(&file_a, &file_b).unwrap();
    let stats = sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(None),
        &opts,
    )
    .unwrap();
    assert_eq!(stats.bytes_transferred, 0);
    assert!(dst.join("b.bin").exists());
}
