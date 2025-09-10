// crates/cli/tests/help.rs
use oc_rsync_cli::{branding, cli_command, render_help};
use serial_test::serial;
use std::env;

fn set_env_var(key: &str, val: &str) {
    env::set_var(key, val);
}

fn remove_env_var(key: &str) {
    env::remove_var(key);
}

fn with_columns<T>(cols: &str, f: impl FnOnce() -> T) -> T {
    let prev = env::var("COLUMNS");
    set_env_var("COLUMNS", cols);
    let out = f();
    match prev {
        Ok(v) => {
            set_env_var("COLUMNS", &v);
        }
        Err(_) => {
            remove_env_var("COLUMNS");
        }
    }
    out
}

#[test]
#[serial]
fn help_columns_80() {
    set_env_var("COLUMNS", "80");
    let out = render_help(&cli_command());
    let mut lines = out.lines();
    assert_eq!(
        lines.next().unwrap(),
        format!("{} {}", branding::program_name(), branding::brand_version())
    );
    assert_eq!(lines.next().unwrap(), branding::brand_tagline());
    assert!(lines.next().unwrap().is_empty());
    assert!(out.contains("Usage:"));
    assert!(out.contains("--verbose, -v"));
    remove_env_var("COLUMNS");
}

#[test]
#[serial]
fn help_columns_120() {
    set_env_var("COLUMNS", "120");
    let out = render_help(&cli_command());
    let mut lines = out.lines();
    assert_eq!(
        lines.next().unwrap(),
        format!("{} {}", branding::program_name(), branding::brand_version())
    );
    assert_eq!(lines.next().unwrap(), branding::brand_tagline());
    assert!(lines.next().unwrap().is_empty());
    assert!(out.contains("--verbose, -v"));
    remove_env_var("COLUMNS");
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
    assert!(env::var("COLUMNS").is_err());
}
