#![cfg(unix)]

use engine::preallocate;
use tempfile::NamedTempFile;

#[test]
fn preallocate_sets_file_length() {
    let tmp = NamedTempFile::new().unwrap();
    let file = tmp.reopen().unwrap();
    let size = 4096;
    preallocate(&file, size).expect("preallocate failed");
    assert_eq!(file.metadata().unwrap().len(), size);
}
