// tools/flag_matrix.rs
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::process::Command;

use serde::Serialize;

#[derive(Serialize)]
struct Entry {
    flag: String,
    status: String,
    notes: String,
}

fn clean_flag(token: &str) -> String {
    let token = token.split_whitespace().next().unwrap_or("");
    let token = token.split('=').next().unwrap_or(token);
    let token = token.trim_end_matches('.');
    token.to_string()
}

fn parse_help(
    text: &str,
) -> (
    BTreeSet<String>,
    HashMap<String, String>,
    HashMap<String, String>,
) {
    let mut flags = BTreeSet::new();
    let mut aliases = HashMap::new();
    let mut alias_desc = HashMap::new();

    for line in text.lines() {
        let line = line.trim_start();
        if !line.starts_with('-') && !line.starts_with("--") {
            continue;
        }
        let mut parts = line.splitn(2, "  ");
        let flags_part = parts.next().unwrap_or("");
        let desc = parts.next().unwrap_or("").trim();
        let raw_tokens: Vec<&str> = flags_part
            .split(',')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .collect();
        if raw_tokens.is_empty() {
            continue;
        }
        let tokens: Vec<String> = raw_tokens.iter().map(|t| clean_flag(t)).collect();
        let mut canonical = tokens
            .iter()
            .filter(|t| t.starts_with("--"))
            .max_by_key(|t| t.len())
            .cloned();
        if desc.contains("alias for") {
            if let Some(idx) = desc.find("--") {
                canonical = Some(clean_flag(&desc[idx..]));
            }
        }
        let Some(canonical) = canonical else {
            continue;
        };
        flags.insert(canonical.clone());
        if desc.contains("alias for") || desc.contains("same as") {
            alias_desc.insert(canonical.clone(), desc.to_string());
        }
        for alias in tokens {
            if alias == canonical {
                continue;
            }
            aliases.insert(alias, canonical.clone());
        }
    }

    (flags, aliases, alias_desc)
}

fn parse_feature_matrix() -> BTreeSet<String> {
    let text = fs::read_to_string("docs/feature_matrix.md").unwrap_or_default();
    let mut ignored = BTreeSet::new();
    for line in text.lines() {
        if !line.trim_start().starts_with('|') {
            continue;
        }
        let mut parts = line.split('|');
        let _ = parts.next();
        let option = match parts.next() {
            Some(s) => s.trim().trim_matches('`').to_string(),
            None => continue,
        };
        let _short = parts.next();
        let supported = match parts.next() {
            Some(s) => s.trim(),
            None => continue,
        };
        let _parity = parts.next();
        let _tests = parts.next();
        let notes = match parts.next() {
            Some(s) => s.trim().to_string(),
            None => String::new(),
        };
        if supported != "✅"
            || notes.contains("requires `acl` feature")
            || notes.contains("requires `xattr` feature")
        {
            ignored.insert(option);
        }
    }
    ignored
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rsync_help_str = fs::read_to_string("tests/fixtures/rsync-help.txt")?;
    let (rsync_flags, rsync_aliases, rsync_alias_desc) = parse_help(&rsync_help_str);

    let oc_rsync_help = Command::new("cargo")
        .args(["run", "--quiet", "--bin", "oc-rsync", "--", "--help"])
        .output()?;
    let oc_rsync_help_str = String::from_utf8(oc_rsync_help.stdout)?;
    let (mut oc_rsync_flags, oc_rsync_aliases, _oc_rsync_alias_desc) =
        parse_help(&oc_rsync_help_str);
    if Command::new("cargo")
        .args(["run", "--quiet", "--bin", "oc-rsync", "--", "--version"])
        .output()?
        .status
        .success()
    {
        oc_rsync_flags.insert("--version".to_string());
    }

    let error_notes: HashMap<&str, &str> = [].into_iter().collect();
    let ignored_flags = parse_feature_matrix();

    let mut entries = Vec::new();
    for flag in rsync_flags.iter() {
        let mut status = if oc_rsync_flags.contains(flag) {
            if error_notes.contains_key(flag.as_str()) {
                "Error"
            } else if oc_rsync_aliases.contains_key(flag) {
                "Alias"
            } else {
                "Supported"
            }
        } else {
            "Error"
        };
        if ignored_flags.contains(flag) {
            status = "Ignored";
        }
        let mut notes = String::new();
        if let Some(desc) = rsync_alias_desc.get(flag) {
            notes.push_str(desc);
        } else if let Some(canon) = rsync_aliases.get(flag) {
            notes.push_str(&format!("alias for {}", canon));
        }
        if let Some(err) = error_notes.get(flag.as_str()) {
            if !notes.is_empty() {
                notes.push_str("; ");
            }
            notes.push_str(err);
            status = "Error";
        }
        entries.push(Entry {
            flag: flag.clone(),
            status: status.to_string(),
            notes,
        });
    }

    entries.sort_by(|a, b| a.flag.cmp(&b.flag));

    let json = serde_json::to_string_pretty(&entries)?;
    fs::write("tools/flag_matrix.json", json)?;

    let mut md = String::from("| flag | status | notes |\n| --- | --- | --- |\n");
    for e in &entries {
        md.push_str(&format!("| {} | {} | {} |\n", e.flag, e.status, e.notes));
    }
    fs::write("tools/flag_matrix.md", md)?;

    if let Some(missing) = entries
        .iter()
        .filter(|e| e.status == "Error")
        .map(|e| e.flag.clone())
        .reduce(|mut acc, f| {
            acc.push_str(", ");
            acc.push_str(&f);
            acc
        })
    {
        eprintln!("missing flags: {}", missing);
        std::process::exit(1);
    }

    Ok(())
}
