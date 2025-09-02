// crates/transport/tests/hosts.rs
use std::net::{Ipv4Addr, Ipv6Addr, TcpListener, TcpStream};
use std::thread;

use transport::tcp::TcpTransport;

#[test]
fn ipv4_cidr_allows() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind");
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let _ = TcpStream::connect(addr);
    });

    let allow = vec!["127.0.0.0/8".to_string()];
    TcpTransport::accept(&listener, &allow, &[]).expect("accept");
}

#[test]
fn ipv6_cidr_allows() {
    let listener = TcpListener::bind((Ipv6Addr::LOCALHOST, 0)).expect("bind");
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let _ = TcpStream::connect(addr);
    });

    let allow = vec!["::1/128".to_string()];
    TcpTransport::accept(&listener, &allow, &[]).expect("accept");
}

#[test]
fn hostname_allows() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind");
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let _ = TcpStream::connect(addr);
    });

    let allow = vec!["localhost".to_string()];
    TcpTransport::accept(&listener, &allow, &[]).expect("accept");
}
