// crates/daemon/tests/config.rs
use daemon::config::validator::parse_bool;
use daemon::{Module, parse_config, parse_module};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[test]
fn parse_config_invalid_port() {
    let cfg = "port=not-a-number";
    let res = parse_config(cfg);
    assert!(res.is_err());
}

#[test]
fn parse_bool_is_case_insensitive() {
    assert!(parse_bool("TRUE").unwrap());
    assert!(parse_bool("Yes").unwrap());
    assert!(!parse_bool("FALSE").unwrap());
    assert!(!parse_bool("No").unwrap());
}

#[test]
fn parse_config_allows_zero_port() {
    let cfg = parse_config("port=0").unwrap();
    assert_eq!(cfg.port, Some(0));
}

#[test]
fn parse_config_accepts_ipv4_address() {
    let cfg = parse_config("address=127.0.0.1").unwrap();
    assert_eq!(cfg.address, Some("127.0.0.1".parse().unwrap()));
    assert!(cfg.address6.is_none());
}

#[test]
fn parse_config_accepts_ipv6_address() {
    let cfg = parse_config("address6=::1").unwrap();
    assert_eq!(cfg.address6, Some("::1".parse().unwrap()));
    assert!(cfg.address.is_none());
}

#[test]
fn parse_config_sets_timeout() {
    let cfg = parse_config("timeout=5").unwrap();
    assert_eq!(cfg.timeout, Some(Duration::from_secs(5)));
}

#[test]
fn parse_config_module_timeout() {
    let cfg = parse_config("[data]\npath=/tmp\ntimeout=7").unwrap();
    assert_eq!(cfg.modules[0].timeout, Some(Duration::from_secs(7)));
}

#[test]
fn parse_config_module_use_chroot() {
    let cfg = parse_config("[data]\npath=/tmp\nuse chroot=no").unwrap();
    assert!(!cfg.modules[0].use_chroot);
}

#[test]
fn parse_config_module_numeric_ids() {
    let cfg = parse_config("[data]\npath=/tmp\nnumeric ids = yes").unwrap();
    assert!(cfg.modules[0].numeric_ids);
}

#[test]
fn parse_config_module_uid_gid() {
    let cfg = parse_config("[data]\npath=/tmp\nuid=0\ngid=0\n").unwrap();
    assert_eq!(cfg.modules[0].uid, Some(0));
    assert_eq!(cfg.modules[0].gid, Some(0));
}

#[test]
fn parse_config_global_uid_gid() {
    let cfg = parse_config("uid=0\ngid=0\n[data]\npath=/tmp\n").unwrap();
    assert_eq!(cfg.uid, Some(0));
    assert_eq!(cfg.gid, Some(0));
}

#[test]
fn parse_config_pid_and_lock_file() {
    let cfg = parse_config("pid file=/tmp/pid\nlock file=/tmp/lock").unwrap();
    assert_eq!(cfg.pid_file, Some(PathBuf::from("/tmp/pid")));
    assert_eq!(cfg.lock_file, Some(PathBuf::from("/tmp/lock")));
}

#[test]
fn parse_config_global_use_chroot() {
    let cfg = parse_config("use chroot=no\n[data]\npath=/tmp").unwrap();
    assert_eq!(cfg.use_chroot, Some(false));
}

#[test]
fn parse_config_global_numeric_ids() {
    let cfg = parse_config("numeric ids=yes\n[data]\npath=/tmp").unwrap();
    assert_eq!(cfg.numeric_ids, Some(true));
}

#[cfg(unix)]
#[test]
fn parse_module_accepts_symlinked_dir() {
    let dir = tempdir().unwrap();
    let link = dir.path().join("symlinked");
    symlink(dir.path(), &link).unwrap();
    let module = parse_module(&format!("data={}", link.display())).unwrap();
    assert_eq!(module.path, link);
}

#[cfg(unix)]
#[test]
fn parse_config_resolves_symlinked_path() {
    let dir = tempdir().unwrap();
    let link = dir.path().join("symlinked");
    symlink(dir.path(), &link).unwrap();
    let cfg = parse_config(&format!("[data]\npath={}\n", link.display())).unwrap();
    assert_eq!(cfg.modules[0].path, std::fs::canonicalize(&link).unwrap());
}

#[test]
fn module_builder_defaults() {
    let module = Module::builder("data", "/tmp").build();
    assert!(module.read_only);
    assert!(module.list);
    assert!(module.use_chroot);
    assert!(!module.numeric_ids);
}

#[test]
fn parse_config_module_defaults() {
    let cfg = parse_config("[data]\npath=/tmp").unwrap();
    let module = &cfg.modules[0];
    assert!(module.read_only);
    assert!(module.list);
    assert!(module.use_chroot);
}
