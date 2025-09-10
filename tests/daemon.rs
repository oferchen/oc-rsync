// tests/daemon.rs

use daemon::{parse_config, parse_module};
use std::fs;
use std::time::Duration;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;

#[test]
fn parse_module_parses_options() {
    let dir = tempdir().unwrap();
    let auth = dir.path().join("auth");
    fs::write(&auth, "alice data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&auth, fs::Permissions::from_mode(0o600)).unwrap();
    let spec = format!(
        "data={},comment=hi,write-only=yes,hosts-allow=127.0.0.1,hosts-deny=10.0.0.1,auth-users=alice bob,secrets-file={},uid=0,gid=0,timeout=1,use-chroot=no,numeric-ids=yes",
        dir.path().display(),
        auth.display()
    );
    let module = parse_module(&spec).unwrap();
    assert_eq!(module.name, "data");
    assert_eq!(module.path, fs::canonicalize(dir.path()).unwrap());
    assert_eq!(module.comment.as_deref(), Some("hi"));
    assert_eq!(module.hosts_allow, vec!["127.0.0.1".to_string()]);
    assert_eq!(module.hosts_deny, vec!["10.0.0.1".to_string()]);
    assert_eq!(
        module.auth_users,
        vec!["alice".to_string(), "bob".to_string()]
    );
    assert_eq!(module.secrets_file, Some(PathBuf::from(&auth)));
    assert_eq!(module.uid, Some(0));
    assert_eq!(module.gid, Some(0));
    assert_eq!(module.timeout, Some(Duration::from_secs(1)));
    assert!(!module.use_chroot);
    assert!(module.numeric_ids);
    assert!(module.write_only);
}

#[test]
fn parse_module_rejects_missing_path() {
    assert!(parse_module("data=").is_err());
    assert!(parse_module("data=,comment=hi").is_err());
}

#[cfg(unix)]
#[test]
fn parse_module_resolves_named_uid_gid() {
    let group_name = users::get_group_by_gid(0)
        .expect("group for gid 0")
        .name()
        .to_string_lossy()
        .into_owned();
    let spec = format!("data=/tmp,uid=root,gid={group_name}");
    let module = parse_module(&spec).unwrap();
    assert_eq!(module.uid, Some(0));
    assert_eq!(module.gid, Some(0));
}

#[test]
fn parse_config_accepts_hyphenated_keys() {
    let cfg = parse_config("motd-file=/tmp/m\n[data]\npath=/tmp\n").unwrap();
    assert_eq!(cfg.motd_file, Some(PathBuf::from("/tmp/m")));
}

#[test]
fn parse_config_requires_module_path() {
    assert!(parse_config("[data]\nuid=0\n").is_err());
}

#[cfg(unix)]
#[test]
fn parse_config_resolves_symlink_path() {
    let dir = tempdir().unwrap();
    let link = dir.path().join("link");
    symlink(dir.path(), &link).unwrap();
    let cfg = parse_config(&format!("[data]\npath={}\n", link.display())).unwrap();
    assert_eq!(cfg.modules[0].path, fs::canonicalize(&link).unwrap());
}
