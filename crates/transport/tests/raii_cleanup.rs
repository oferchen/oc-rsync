use std::fs;
use tempfile::tempdir;
use transport::{TempFileGuard, TempSocketGuard};

#[test]
fn temp_file_guard_removes_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("temp_file");
    fs::write(&path, b"data").unwrap();
    let guard = TempFileGuard::new(&path);
    assert!(path.exists());
    drop(guard);
    assert!(!path.exists());
}

#[test]
fn temp_socket_guard_removes_path() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("temp_sock");
    fs::write(&path, b"data").unwrap();
    let guard = TempSocketGuard::new(&path);
    assert!(path.exists());
    drop(guard);
    assert!(!path.exists());
}
