// crates/logging/tests/log_file_error.rs

use logging::{subscriber, SubscriberConfig};
use tempfile::tempdir;

#[test]
fn log_file_error_propagates() {
    let dir = tempdir().unwrap();
    let cfg = SubscriberConfig::builder()
        .log_file(Some((dir.path().to_path_buf(), None)))
        .build();
    assert!(subscriber(cfg).is_err());
}
