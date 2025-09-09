// crates/transport/tests/factory.rs
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
    unsafe {
        env::set_var("PATH", "");
    }
    let res = TransportFactory::from_uri("ssh://example.com");
    unsafe {
        env::set_var("PATH", old_path);
    }
    let err = match res {
        Ok(_) => panic!("expected error"),
        Err(e) => e,
    };
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
}
