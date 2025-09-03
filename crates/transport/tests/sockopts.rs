// crates/transport/tests/sockopts.rs
use transport::{parse_sockopts, SockOpt};

#[test]
fn parse_ip_ttl() {
    let opts = parse_sockopts(&["ip:ttl=64".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::IpTtl(64)]);
}

#[test]
fn parse_keepalive_enabled() {
    let opts = parse_sockopts(&["SO_KEEPALIVE".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::KeepAlive(true)]);
}

#[test]
fn parse_keepalive_disabled() {
    let opts = parse_sockopts(&["SO_KEEPALIVE=0".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::KeepAlive(false)]);
}

#[test]
fn parse_sndbuf() {
    let opts = parse_sockopts(&["SO_SNDBUF=8192".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::SendBuf(8192)]);
}

#[test]
fn parse_rcvbuf() {
    let opts = parse_sockopts(&["SO_RCVBUF=4096".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::RecvBuf(4096)]);
}

#[test]
fn parse_tcp_nodelay_default() {
    let opts = parse_sockopts(&["TCP_NODELAY".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::TcpNoDelay(true)]);
}

#[test]
fn parse_tcp_nodelay_off() {
    let opts = parse_sockopts(&["TCP_NODELAY=0".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::TcpNoDelay(false)]);
}

#[test]
fn parse_reuseaddr_default() {
    let opts = parse_sockopts(&["SO_REUSEADDR".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::ReuseAddr(true)]);
}

#[test]
fn parse_reuseaddr_off() {
    let opts = parse_sockopts(&["SO_REUSEADDR=0".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::ReuseAddr(false)]);
}

#[test]
fn parse_bindtodevice() {
    let opts = parse_sockopts(&["SO_BINDTODEVICE=eth0".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::BindToDevice("eth0".into())]);
}

#[test]
fn parse_ip_hoplimit() {
    let opts = parse_sockopts(&["ip:hoplimit=5".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::IpHopLimit(5)]);
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

#[test]
fn parse_sndbuf_invalid() {
    assert!(parse_sockopts(&["SO_SNDBUF=abc".into()]).is_err());
}

#[test]
fn parse_rcvbuf_missing() {
    assert!(parse_sockopts(&["SO_RCVBUF".into()]).is_err());
}

#[test]
fn parse_tcp_nodelay_invalid() {
    assert!(parse_sockopts(&["TCP_NODELAY=foo".into()]).is_err());
}
