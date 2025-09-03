// crates/cli/tests/branding.rs
use oc_rsync_cli::{cli_command, render_help};

#[test]
fn help_uses_program_name() {
    std::env::set_var("PROGRAM_NAME", "myrsync");
    std::env::set_var("COLUMNS", "80");
    let cmd = cli_command();
    let help = render_help(&cmd);
    assert!(help.contains("Usage: myrsync"));
    std::env::remove_var("PROGRAM_NAME");
    std::env::remove_var("COLUMNS");
}
