// crates/logging/tests/log_file_error.rs

use logging::{SubscriberConfig, subscriber};
use tempfile::tempdir;

#[test]
fn log_file_error_propagates() {
    let dir = tempdir().unwrap();
    let cfg = SubscriberConfig::builder()
        .log_file(Some((dir.path().to_path_buf(), None)))
        .build();
    assert!(subscriber(cfg).is_err());
}

#[test]
fn missing_parent_dir_fails() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("missing").join("log.txt");
    let cfg = SubscriberConfig::builder()
        .log_file(Some((path, None)))
        .build();
    assert!(subscriber(cfg).is_err());
}
