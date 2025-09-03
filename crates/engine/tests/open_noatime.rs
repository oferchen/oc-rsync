// crates/engine/tests/open_noatime.rs
#![cfg(target_os = "linux")]

use std::fs;
use std::time::SystemTime;

use compress::available_codecs;
use engine::{sync, SyncOptions};
use filetime::{set_file_times, FileTime};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn open_noatime_preserves_source_access_time() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file.txt");
    fs::write(&file, b"hi").unwrap();

    let epoch_atime = FileTime::from_unix_time(0, 0);
    let mtime = FileTime::from_system_time(SystemTime::now());
    set_file_times(&file, epoch_atime, mtime).unwrap();

    let opts = SyncOptions {
        open_noatime: true,
        ..Default::default()
    };
    sync(&src, &dst, &Matcher::default(), &available_codecs(), &opts).unwrap();

    let meta = fs::metadata(&file).unwrap();
    let new_atime = FileTime::from_last_access_time(&meta);
    assert_eq!(new_atime.unix_seconds(), epoch_atime.unix_seconds());
}
