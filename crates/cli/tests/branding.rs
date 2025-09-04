// crates/cli/tests/branding.rs
use oc_rsync_cli::{branding, cli_command, render_help};
use serial_test::serial;

#[test]
#[serial]
fn help_uses_program_name() {
    std::env::set_var("OC_RSYNC_BRAND_NAME", "myrsync");
    std::env::set_var("COLUMNS", "80");
    let version = branding::brand_version();
    let help = render_help(&cli_command());
    let first = help.lines().next().unwrap();
    assert_eq!(first, format!("myrsync {}", version));
    std::env::remove_var("OC_RSYNC_BRAND_NAME");
    std::env::remove_var("COLUMNS");
}

#[test]
#[serial]
fn upstream_name_does_not_replace_rsyncd_conf() {
    std::env::set_var("OC_RSYNC_UPSTREAM_NAME", "ursync");
    std::env::set_var("COLUMNS", "80");
    let help = render_help(&cli_command());
    assert!(help.contains("rsyncd.conf"));
    assert!(!help.contains("ursyncd.conf"));
    std::env::remove_var("OC_RSYNC_UPSTREAM_NAME");
    std::env::remove_var("COLUMNS");
}

#[test]
#[serial]
fn upstream_name_only_replaces_standalone_rsync() {
    std::env::set_var("OC_RSYNC_UPSTREAM_NAME", "ursync");
    std::env::set_var(
        "OC_RSYNC_HELP_HEADER",
        "rsync rsyncs /path/rsync/bin rsync://host\n",
    );
    std::env::set_var("OC_RSYNC_HELP_FOOTER", "");
    std::env::set_var("COLUMNS", "120");

    let help = render_help(&cli_command());

    assert!(help.contains("ursync rsyncs /path/rsync/bin rsync://host"));
    assert!(!help.contains("ursyncs"));
    assert!(!help.contains("/path/ursync/bin"));
    assert!(!help.contains("ursync://host"));

    std::env::remove_var("OC_RSYNC_UPSTREAM_NAME");
    std::env::remove_var("OC_RSYNC_HELP_HEADER");
    std::env::remove_var("OC_RSYNC_HELP_FOOTER");
    std::env::remove_var("COLUMNS");
}
