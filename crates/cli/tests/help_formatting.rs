// crates/cli/tests/help_formatting.rs
use oc_rsync_cli::{cli_command, dump_help_body, render_help};
use serial_test::serial;
use std::collections::HashSet;
use std::env;

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
fn help_wrapping_matches_upstream() {
    let cmd = cli_command();
    let cases = [
        (
            60,
            include_str!("../../../tests/golden/help/rsync-help-60.txt"),
        ),
        (
            80,
            include_str!("../../../tests/golden/help/rsync-help-80.txt"),
        ),
        (
            100,
            include_str!("../../../tests/golden/help/rsync-help-100.txt"),
        ),
    ];
    for (cols, upstream) in cases {
        env::set_var("COLUMNS", cols.to_string());
        let ours = render_help(&cmd);
        assert_eq!(
            extract_options(&ours),
            extract_options(upstream),
            "options mismatch at width {cols}"
        );
    }
    env::remove_var("COLUMNS");
}

#[test]
#[serial]
fn dump_help_body_matches_render_help() {
    let cmd = cli_command();
    let body = dump_help_body(&cmd);

    env::set_var("COLUMNS", "80");
    let full = render_help(&cmd);
    env::remove_var("COLUMNS");

    assert_eq!(body, extract_options(&full));
}
