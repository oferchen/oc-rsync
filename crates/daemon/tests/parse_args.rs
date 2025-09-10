// crates/daemon/tests/parse_args.rs
use daemon::parse_daemon_args;
use transport::AddressFamily;

#[test]
fn parse_daemon_args_parses_options() {
    let args = vec![
        "--address".to_string(),
        "127.0.0.1".to_string(),
        "--port".to_string(),
        "1234".to_string(),
        "--ipv4".to_string(),
    ];
    let opts = parse_daemon_args(args).unwrap();
    assert_eq!(opts.address, Some("127.0.0.1".parse().unwrap()));
    assert_eq!(opts.port, 1234);
    assert!(matches!(opts.family, Some(AddressFamily::V4)));
}

#[test]
fn parse_daemon_args_rejects_mismatch() {
    let args = vec![
        "--address".to_string(),
        "127.0.0.1".to_string(),
        "--ipv6".to_string(),
    ];
    assert!(parse_daemon_args(args).is_err());
}

#[test]
fn parse_daemon_args_invalid_port() {
    let args = vec!["--port".to_string(), "not-a-number".to_string()];
    assert!(parse_daemon_args(args).is_err());
}
