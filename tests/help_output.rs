// tests/help_output.rs
use assert_cmd::Command;
use oc_rsync_cli::branding;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Deserialize)]
struct FlagDesc {
    flag: String,
    description: String,
}

#[test]
fn help_contains_expected_flags() {
    let expected: Vec<FlagDesc> =
        serde_json::from_str(&fs::read_to_string("tests/fixtures/rsync-help.txt").unwrap())
            .unwrap();

    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("COLUMNS", "80")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .arg("--dump-help-body")
        .output()
        .unwrap();

    let mut actual: HashMap<String, String> = HashMap::new();
    let mut in_options = false;
    let stop_marker = format!("Use \"{} --daemon --help\"", branding::program_name());
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.trim() == "Options" {
            in_options = true;
            continue;
        }
        if !in_options {
            continue;
        }
        if line.starts_with(&stop_marker) {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let trimmed = line.trim_end();
        if let Some(idx) = trimmed.find("  ") {
            let (spec, desc) = trimmed.split_at(idx);
            let flag = spec
                .split(',')
                .find(|s| s.trim_start().starts_with("--"))
                .unwrap_or(spec)
                .trim()
                .to_string();
            let desc = desc.split_whitespace().collect::<String>();
            actual.insert(flag, desc);
        }
    }

    for FlagDesc { flag, description } in expected {
        let expected_desc = description.split_whitespace().collect::<String>();
        let actual_desc = actual
            .get(&flag)
            .unwrap_or_else(|| panic!("missing flag {}", flag));
        assert_eq!(
            actual_desc, &expected_desc,
            "description mismatch for {}",
            flag
        );
    }
}
#[test]
fn help_matches_snapshot() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("COLUMNS", "80")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .arg("--dump-help-body")
        .output()
        .unwrap();

    let actual = output.stdout;
    let expected = fs::read("tests/golden/help/oc-rsync.help").unwrap();
    assert_eq!(actual, expected, "help output does not match snapshot");
}

#[test]
fn dump_help_body_60_matches_golden() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("COLUMNS", "60")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .arg("--dump-help-body")
        .output()
        .unwrap();

    let expected = fs::read("tests/golden/help/oc-rsync.dump-help-body.60").unwrap();
    assert_eq!(output.stdout, expected, "dump-help-body width 60 mismatch");
}

#[test]
fn dump_help_body_100_matches_golden() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("COLUMNS", "100")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .arg("--dump-help-body")
        .output()
        .unwrap();

    let expected = fs::read("tests/golden/help/oc-rsync.dump-help-body.100").unwrap();
    assert_eq!(output.stdout, expected, "dump-help-body width 100 mismatch");
}
