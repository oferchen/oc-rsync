// crates/cli/tests/branding.rs
use oc_rsync_cli::{branding, cli_command, render_help};
use serial_test::serial;
use std::env;
use std::sync::{Mutex, OnceLock};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn set_env_var(key: &str, val: &str) {
    env::set_var(key, val);
}

fn remove_env_var(key: &str) {
    env::remove_var(key);
}

#[test]
#[serial]
fn help_uses_program_name() {
    set_env_var("OC_RSYNC_NAME", "myrsync");
    set_env_var("COLUMNS", "80");
    let version = branding::brand_version();
    let help = render_help(&cli_command());
    let first = help.lines().next().unwrap();
    assert_eq!(first, format!("myrsync {}", version));
    remove_env_var("OC_RSYNC_NAME");
    remove_env_var("COLUMNS");
}

#[test]
#[serial]
fn upstream_name_does_not_replace_rsyncd_conf() {
    set_env_var("OC_RSYNC_UPSTREAM_NAME", "ursync");
    set_env_var("COLUMNS", "80");
    let help = render_help(&cli_command());
    assert!(help.contains("rsyncd.conf"));
    assert!(!help.contains("ursyncd.conf"));
    remove_env_var("OC_RSYNC_UPSTREAM_NAME");
    remove_env_var("COLUMNS");
}

#[test]
#[serial]
fn upstream_name_only_replaces_standalone_rsync() {
    set_env_var("OC_RSYNC_UPSTREAM_NAME", "ursync");
    set_env_var(
        "OC_RSYNC_HELP_HEADER",
        "rsync rsyncs /path/rsync/bin rsync://host\n",
    );
    set_env_var("OC_RSYNC_HELP_FOOTER", "");
    set_env_var("COLUMNS", "120");

    let help = render_help(&cli_command());

    assert!(help.contains("ursync rsyncs /path/rsync/bin rsync://host"));
    assert!(!help.contains("ursyncs"));
    assert!(!help.contains("/path/ursync/bin"));
    assert!(!help.contains("ursync://host"));

    remove_env_var("OC_RSYNC_UPSTREAM_NAME");
    remove_env_var("OC_RSYNC_HELP_HEADER");
    remove_env_var("OC_RSYNC_HELP_FOOTER");
    remove_env_var("COLUMNS");
}
