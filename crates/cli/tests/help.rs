// crates/cli/tests/help.rs
use oc_rsync_cli::{cli_command, render_help};
use serial_test::serial;
use std::env;

#[test]
#[serial]
fn help_columns_80() {
    env::set_var("COLUMNS", "80");
    let out = render_help(&cli_command());
    insta::assert_snapshot!("help_columns_80", out);
}

#[test]
#[serial]
fn help_columns_120() {
    env::set_var("COLUMNS", "120");
    let out = render_help(&cli_command());
    insta::assert_snapshot!("help_columns_120", out);
}
