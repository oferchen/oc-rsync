// crates/engine/tests/write_devices.rs
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::FileTypeExt;

use compress::available_codecs;
use engine::{sync, SyncOptions};
use filters::Matcher;
use meta::makedev;
use nix::sys::stat::{mknod, Mode, SFlag};
use tempfile::tempdir;

#[test]
fn requires_flag_to_write_devices() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    let dev = dst.join("file");
    mknod(
        &dev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        makedev(1, 3),
    )
    .unwrap();

    let err = sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions::default(),
    )
    .unwrap_err();
    assert!(format!("{}", err).contains("refusing to write to device"));

    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            write_devices: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::symlink_metadata(dev).unwrap();
    assert!(meta.file_type().is_char_device());
}
