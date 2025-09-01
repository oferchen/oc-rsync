// crates/transport/tests/sockopts.rs
use transport::{parse_sockopts, SockOpt};

#[test]
fn parse_ip_ttl() {
    let opts = parse_sockopts(&["ip:ttl=64".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::IpTtl(64)]);
}

#[test]
fn parse_ip_tos_hex() {
    let opts = parse_sockopts(&["ip:tos=0x10".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::IpTos(0x10)]);
}

#[test]
fn parse_ip_unknown() {
    assert!(parse_sockopts(&["ip:foo=1".into()]).is_err());
}

#[test]
fn parse_ip_missing_value() {
    assert!(parse_sockopts(&["ip:ttl".into()]).is_err());
}
