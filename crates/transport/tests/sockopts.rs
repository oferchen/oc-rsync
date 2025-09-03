// crates/transport/tests/sockopts.rs
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use socket2::SockRef;
use transport::{parse_sockopts, tcp::TcpTransport, SockOpt};

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
fn parse_linger() {
    let opts = parse_sockopts(&["SO_LINGER=5".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::Linger(Some(Duration::from_secs(5)))]);
}

#[test]
fn parse_broadcast_default() {
    let opts = parse_sockopts(&["SO_BROADCAST".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::Broadcast(true)]);
}

#[test]
fn parse_broadcast_off() {
    let opts = parse_sockopts(&["SO_BROADCAST=0".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::Broadcast(false)]);
}

#[test]
fn parse_rcvtimeout() {
    let opts = parse_sockopts(&["SO_RCVTIMEO=10".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::RcvTimeout(Duration::from_secs(10))]);
}

#[test]
fn parse_sndtimeout() {
    let opts = parse_sockopts(&["SO_SNDTIMEO=12".into()]).unwrap();
    assert_eq!(opts, vec![SockOpt::SndTimeout(Duration::from_secs(12))]);
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

#[test]
fn apply_sockopts_linger_broadcast_timeouts() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    thread::spawn(move || {
        let _ = listener.accept().unwrap();
    });

    let stream = TcpStream::connect(addr).unwrap();
    let inspect = stream.try_clone().unwrap();
    let transport = TcpTransport::from_stream(stream);

    let opts = vec![
        SockOpt::Linger(Some(Duration::from_secs(5))),
        SockOpt::Broadcast(true),
        SockOpt::RcvTimeout(Duration::from_secs(10)),
        SockOpt::SndTimeout(Duration::from_secs(12)),
    ];
    transport.apply_sockopts(&opts).unwrap();

    let sock = SockRef::from(&inspect);
    assert_eq!(sock.linger().unwrap(), Some(Duration::from_secs(5)));
    assert!(sock.broadcast().unwrap());
    assert_eq!(sock.read_timeout().unwrap(), Some(Duration::from_secs(10)));
    assert_eq!(sock.write_timeout().unwrap(), Some(Duration::from_secs(12)));
}

#[cfg(target_os = "linux")]
#[test]
fn apply_sockopts_ipv4_basic() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    thread::spawn(move || {
        let _ = listener.accept().unwrap();
    });

    let stream = TcpStream::connect(addr).unwrap();
    let inspect = stream.try_clone().unwrap();
    let transport = TcpTransport::from_stream(stream);

    let opts = vec![
        SockOpt::KeepAlive(true),
        SockOpt::SendBuf(8192),
        SockOpt::RecvBuf(4096),
        SockOpt::TcpNoDelay(true),
        SockOpt::ReuseAddr(true),
        SockOpt::IpTtl(55),
        SockOpt::IpTos(0x10),
    ];
    transport.apply_sockopts(&opts).unwrap();

    let sock = SockRef::from(&inspect);
    assert!(sock.keepalive().unwrap());
    assert!(sock.nodelay().unwrap());
    assert!(sock.reuse_address().unwrap());
    assert!(sock.send_buffer_size().unwrap() >= 8192);
    assert!(sock.recv_buffer_size().unwrap() >= 4096);
    assert_eq!(sock.ttl().unwrap(), 55);
    assert_eq!(sock.tos().unwrap(), 0x10);
}

#[cfg(target_os = "linux")]
#[test]
fn apply_sockopts_ipv6_hoplimit() {
    let listener = TcpListener::bind("[::1]:0").unwrap();
    let addr = listener.local_addr().unwrap();

    thread::spawn(move || {
        let _ = listener.accept().unwrap();
    });

    let stream = TcpStream::connect(addr).unwrap();
    let inspect = stream.try_clone().unwrap();
    let transport = TcpTransport::from_stream(stream);

    let opts = vec![SockOpt::IpHopLimit(7)];
    transport.apply_sockopts(&opts).unwrap();

    let sock = SockRef::from(&inspect);
    assert_eq!(sock.unicast_hops_v6().unwrap(), 7);
}

#[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
#[test]
fn apply_sockopt_bind_device() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    thread::spawn(move || {
        let _ = listener.accept().unwrap();
    });

    let stream = TcpStream::connect(addr).unwrap();
    let transport = TcpTransport::from_stream(stream);

    transport
        .apply_sockopts(&[SockOpt::BindToDevice("lo".into())])
        .unwrap();
}

#[cfg(not(any(target_os = "android", target_os = "fuchsia", target_os = "linux")))]
#[test]
fn apply_sockopt_bind_device_unsupported() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    thread::spawn(move || {
        let _ = listener.accept().unwrap();
    });

    let stream = TcpStream::connect(addr).unwrap();
    let transport = TcpTransport::from_stream(stream);

    assert!(transport
        .apply_sockopts(&[SockOpt::BindToDevice("eth0".into())])
        .is_err());
}
