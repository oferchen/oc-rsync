// crates/cli/tests/branding.rs
use oc_rsync_cli::{branding, cli_command, render_help};
use serial_test::serial;

#[test]
#[serial]
fn help_uses_program_name() {
    temp_env::with_vars(
        [
            ("OC_RSYNC_NAME", Some("myrsync")),
            ("COLUMNS", Some("80")),
        ],
        || {
            let version = branding::brand_version();
            let help = render_help(&cli_command());
            let first = help.lines().next().unwrap();
            assert_eq!(first, format!("myrsync {}", version));
        },
    );
}

#[test]
#[serial]
fn upstream_name_does_not_replace_rsyncd_conf() {
    temp_env::with_vars(
        [
            ("OC_RSYNC_UPSTREAM_NAME", Some("ursync")),
            ("COLUMNS", Some("80")),
        ],
        || {
            let help = render_help(&cli_command());
            assert!(help.contains("rsyncd.conf"));
            assert!(!help.contains("ursyncd.conf"));
        },
    );
}

#[test]
#[serial]
fn upstream_name_only_replaces_standalone_rsync() {
    temp_env::with_vars(
        [
            ("OC_RSYNC_UPSTREAM_NAME", Some("ursync")),
            (
                "OC_RSYNC_HELP_HEADER",
                Some("rsync rsyncs /path/rsync/bin rsync://host\n"),
            ),
            ("OC_RSYNC_HELP_FOOTER", Some("")),
            ("COLUMNS", Some("120")),
        ],
        || {
            let help = render_help(&cli_command());

            assert!(help.contains("ursync rsyncs /path/rsync/bin rsync://host"));
            assert!(!help.contains("ursyncs"));
            assert!(!help.contains("/path/ursync/bin"));
            assert!(!help.contains("ursync://host"));
        },
    );
}
