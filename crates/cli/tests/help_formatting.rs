// crates/cli/tests/help_formatting.rs
use oc_rsync_cli::{cli_command, dump_help_body, render_help};
use serial_test::serial;
use std::collections::HashSet;

fn extract_options(help: &str) -> String {
    let mut out = String::new();
    let mut in_opts = false;
    let stop_marker = "Use \"rsync --daemon --help\"";
    for line in help.lines() {
        if line.trim() == "Options" {
            in_opts = true;
            continue;
        }
        if !in_opts {
            continue;
        }
        if line.starts_with(stop_marker) {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[test]
fn dump_help_body_lists_unique_options() {
    let output = dump_help_body(&cli_command());
    let mut seen = HashSet::new();
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, '\t');
        let flag = parts.next().unwrap();
        assert!(seen.insert(flag.to_string()), "duplicate flag {flag}");
    }
}

#[test]
#[serial]
fn help_wrapping_matches_upstream_80() {
    let cmd = cli_command();
    let ours = temp_env::with_var("COLUMNS", Some("80"), || render_help(&cmd));
    let upstream = include_str!("../../../tests/golden/help/rsync-help-80.txt");
    assert_eq!(extract_options(&ours), extract_options(upstream));
}

#[test]
#[serial]
fn help_wrapping_matches_upstream_100() {
    let cmd = cli_command();
    let ours = temp_env::with_var("COLUMNS", Some("100"), || render_help(&cmd));
    let upstream = include_str!("../../../tests/golden/help/rsync-help-100.txt");
    assert_eq!(extract_options(&ours), extract_options(upstream));
}

#[test]
#[serial]
fn dump_help_body_matches_render_help() {
    let cmd = cli_command();
    let body = dump_help_body(&cmd);

    let full = temp_env::with_var("COLUMNS", Some("80"), || render_help(&cmd));

    assert_eq!(body, extract_options(&full));
}
