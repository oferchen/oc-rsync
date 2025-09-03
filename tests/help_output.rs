// tests/help_output.rs
use assert_cmd::Command;
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
        .arg("--help")
        .output()
        .unwrap();

    let mut actual: HashMap<String, String> = HashMap::new();
    let mut in_options = false;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.trim() == "Options" {
            in_options = true;
            continue;
        }
        if !in_options {
            continue;
        }
        if line.starts_with("Use \"rsync --daemon --help\"") {
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
fn help_matches_upstream() {
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("COLUMNS", "80")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .arg("--help")
        .output()
        .unwrap();

    let mut ours = String::from_utf8(output.stdout).unwrap();
    ours = ours.replace("oc-rsync", "rsync");
    let expected = fs::read_to_string("crates/cli/resources/rsync-help-80.txt").unwrap();
    assert_eq!(ours, expected, "help output diverges from upstream");
}
