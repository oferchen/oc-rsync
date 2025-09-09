// crates/logging/tests/log_file.rs

use logging::{DebugFlag, InfoFlag, LogFormat, SubscriberConfig, init};
use tempfile::tempdir;
use tracing::info;

#[test]
fn file_sink_writes_message() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("log.txt");
    let cfg = SubscriberConfig::builder()
        .format(LogFormat::Text)
        .verbose(1)
        .info(&[] as &[InfoFlag])
        .debug(&[] as &[DebugFlag])
        .quiet(false)
        .log_file(Some((path.clone(), None)))
        .syslog(false)
        .journald(false)
        .colored(false)
        .timestamps(false)
        .build();
    init(cfg).unwrap();
    info!(target: "test", "hello");
    let contents = std::fs::read_to_string(path).unwrap();
    assert!(contents.contains("hello"));
}
