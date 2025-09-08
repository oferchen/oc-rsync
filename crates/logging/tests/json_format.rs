// crates/logging/tests/json_format.rs
use logging::{LogFormat, SubscriberConfig, subscriber};
use serde_json::Value;
use std::fs;
use tempfile::tempdir;
use tracing::info;

#[test]
fn json_formatting_works() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("log.json");
    let cfg = SubscriberConfig::builder()
        .format(LogFormat::Json)
        .quiet(true)
        .log_file(Some((path.clone(), Some("json".to_string()))))
        .build();
    let subscriber = subscriber(cfg).unwrap();
    tracing::subscriber::with_default(subscriber, || {
        info!(target: "test", foo = 1, "hello");
    });
    let contents = fs::read_to_string(path).unwrap();
    let v: Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(v["fields"]["message"], "hello");
    assert_eq!(v["fields"]["foo"], 1);
}
