// crates/transport/tests/config.rs
use std::time::Duration;

use transport::TransportConfig;

#[test]
fn builder_provides_defaults() {
    let cfg = TransportConfig::builder().build().unwrap();
    assert_eq!(cfg.timeout, Some(Duration::from_secs(30)));
    assert_eq!(cfg.retries, 3);
    assert!(cfg.rate_limit.is_none());
}

#[test]
fn builder_rejects_zero_timeout() {
    let res = TransportConfig::builder()
        .timeout(Duration::from_secs(0))
        .build();
    assert!(res.is_err());
}

#[test]
fn builder_rejects_zero_rate_limit() {
    let res = TransportConfig::builder().rate_limit(0).build();
    assert!(res.is_err());
}
