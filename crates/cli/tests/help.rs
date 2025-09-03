// crates/cli/tests/help.rs
use oc_rsync_cli::{branding, cli_command, render_help};
use serial_test::serial;
use std::env;

#[test]
#[serial]
fn help_columns_80() {
    env::set_var("COLUMNS", "80");
    let out = render_help(&cli_command());
    let mut lines = out.lines();
    assert_eq!(
        lines.next().unwrap(),
        format!("{} {}", branding::program_name(), branding::brand_version())
    );
    assert_eq!(lines.next().unwrap(), branding::brand_credits());
    assert!(lines.next().unwrap().is_empty());
    let body = lines.collect::<Vec<_>>().join("\n");
    let expected = include_str!("../resources/rsync-help-80.txt").trim_end();
    assert_eq!(body, expected);
}

#[test]
#[serial]
fn help_columns_120() {
    env::set_var("COLUMNS", "120");
    let out = render_help(&cli_command());
    let mut lines = out.lines();
    assert_eq!(
        lines.next().unwrap(),
        format!("{} {}", branding::program_name(), branding::brand_version())
    );
    assert_eq!(lines.next().unwrap(), branding::brand_credits());
    assert!(lines.next().unwrap().is_empty());
    let body = lines.collect::<Vec<_>>().join("\n");
    let expected = include_str!("../../../tests/fixtures/rsync-help-120.txt").trim_end();
    assert_eq!(body, expected);
}

#[test]
#[serial]
fn help_columns_small() {
    env::set_var("COLUMNS", "25");
    let out = render_help(&cli_command());
    assert!(!out.is_empty());
}
