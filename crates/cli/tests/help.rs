// crates/cli/tests/help.rs
use oc_rsync_cli::{branding, cli_command, render_help};
use serial_test::serial;

fn with_columns<T>(cols: &str, f: impl FnOnce() -> T) -> T {
    temp_env::with_var("COLUMNS", Some(cols), f)
}

#[test]
#[serial]
fn help_columns_80() {
    let out = temp_env::with_var("COLUMNS", Some("80"), || render_help(&cli_command()));
    let mut lines = out.lines();
    assert_eq!(
        lines.next().unwrap(),
        format!("{} {}", branding::program_name(), branding::brand_version())
    );
    assert_eq!(lines.next().unwrap(), branding::brand_tagline());
    assert!(lines.next().unwrap().is_empty());
    assert!(out.contains("Usage:"));
    assert!(out.contains("--verbose, -v"));
}

#[test]
#[serial]
fn help_columns_120() {
    let out = temp_env::with_var("COLUMNS", Some("120"), || render_help(&cli_command()));
    let mut lines = out.lines();
    assert_eq!(
        lines.next().unwrap(),
        format!("{} {}", branding::program_name(), branding::brand_version())
    );
    assert_eq!(lines.next().unwrap(), branding::brand_tagline());
    assert!(lines.next().unwrap().is_empty());
    assert!(out.contains("--verbose, -v"));
}

#[test]
#[serial]
fn help_columns_small() {
    with_columns("25", || {
        let out = render_help(&cli_command());
        assert!(!out.is_empty());
    })
}

#[test]
#[serial]
fn help_columns_small_env_restored() {
    assert!(std::env::var("COLUMNS").is_err());
}
