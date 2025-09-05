// crates/engine/tests/auto_tmp.rs
use std::fs;
use std::thread;
use std::time::Duration;

use compress::available_codecs;
use engine::{sync, SyncOptions};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn uses_file_stem_for_auto_tmp() {
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

    let tmp_path = dst.join("a.tmp");
    while !handle.is_finished() && !tmp_path.exists() {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(tmp_path.exists());
    handle.join().unwrap();
    assert!(dst.join("a.txt").exists());
    assert!(!tmp_path.exists());
}
