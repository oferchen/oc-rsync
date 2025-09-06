// crates/engine/tests/auto_tmp.rs
use std::fs;
use std::thread;
use std::time::Duration;

use compress::available_codecs;
use engine::{SyncOptions, sync};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn hides_temp_files() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let data = vec![0u8; 100 * 1024 * 1024];
    fs::write(src.join("a.txt"), &data).unwrap();

    let handle = {
        let src = src.clone();
        let dst = dst.clone();
        thread::spawn(move || {
            sync(
                &src,
                &dst,
                &Matcher::default(),
                &available_codecs(),
                &SyncOptions::default(),
            )
            .unwrap();
        })
    };

    while !handle.is_finished() {
        assert!(!dst.join("a.txt").exists());
        for entry in fs::read_dir(&dst).unwrap() {
            let entry = entry.unwrap();
            let ty = entry.file_type().unwrap();
            assert!(!ty.is_file());
        }
        thread::sleep(Duration::from_millis(10));
    }
    handle.join().unwrap();
    let entry = fs::read_dir(&dst).unwrap().next().unwrap().unwrap();
    assert_eq!(entry.file_name(), "a.txt");
    assert!(dst.join("a.txt").exists());
    assert!(fs::read_dir(&dst).unwrap().nth(1).is_none());
}
