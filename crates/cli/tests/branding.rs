// crates/cli/tests/branding.rs
use oc_rsync_cli::{branding, cli_command, render_help};

#[test]
fn help_uses_program_name() {
    std::env::set_var("PROGRAM_NAME", "myrsync");
    std::env::set_var("COLUMNS", "80");
    let version = branding::brand_version();
    let help = render_help(&cli_command());
    let first = help.lines().next().unwrap();
    assert_eq!(first, format!("myrsync {}", version));
    std::env::remove_var("PROGRAM_NAME");
    std::env::remove_var("COLUMNS");
}
