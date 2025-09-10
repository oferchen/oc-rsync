// crates/logging/tests/directive_parse_error.rs
use std::io;
use tracing_subscriber::filter::Directive;

#[test]
fn directive_parse_error_propagates() {
    let res: io::Result<Directive> = "info::backup=invalid"
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e));
    assert!(res.is_err());
}
