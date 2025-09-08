use std::env;
use std::io;

use transport::TransportFactory;

#[test]
fn unsupported_scheme() {
    assert!(TransportFactory::from_uri("ftp://example.com").is_err());
}

#[test]
fn ssh_spawn_failure() {
    let old_path = env::var("PATH").unwrap_or_default();
    env::set_var("PATH", "");
    let err = TransportFactory::from_uri("ssh://example.com").unwrap_err();
    env::set_var("PATH", old_path);
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
}
