// crates/cli/tests/remote_spec.rs
use oc_rsync_cli::{RemoteSpec, parse_remote_spec};
use std::ffi::OsStr;
use std::path::Path;

#[test]
fn parses_rsync_module_with_root_path() {
    let spec = parse_remote_spec(OsStr::new("rsync://host/mod/")).unwrap();
    match spec {
        RemoteSpec::Remote { module, path, .. } => {
            assert_eq!(module.as_deref(), Some("mod"));
            assert_eq!(path.path, Path::new("."));
        }
        RemoteSpec::Local(_) => panic!("expected RemoteSpec::Remote"),
    }
}

#[test]
fn parses_rsync_module_with_port() {
    let spec = parse_remote_spec(OsStr::new("rsync://host:1234/mod/")).unwrap();
    match spec {
        RemoteSpec::Remote {
            host,
            port,
            module,
            path,
        } => {
            assert_eq!(host, "host");
            assert_eq!(port, Some(1234));
            assert_eq!(module.as_deref(), Some("mod"));
            assert_eq!(path.path, Path::new("."));
        }
        RemoteSpec::Local(_) => panic!("expected RemoteSpec::Remote"),
    }
}
