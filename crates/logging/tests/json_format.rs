// crates/logging/tests/json_format.rs
use logging::{LogFormat, SubscriberConfig, subscriber};
use serde_json::{Deserializer, Value};
use std::fs::File;
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
        info!(target: "test", bar = 2, "world");
    });
    let file = File::open(path).unwrap();
    let stream = Deserializer::from_reader(file).into_iter::<Value>();
    let values: Vec<Value> = stream.collect::<Result<_, _>>().unwrap();
    assert_eq!(values[0]["fields"]["message"], "hello");
    assert_eq!(values[0]["fields"]["foo"], 1);
    assert_eq!(values[1]["fields"]["message"], "world");
    assert_eq!(values[1]["fields"]["bar"], 2);
}
