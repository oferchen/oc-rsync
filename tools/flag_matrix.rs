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
        let canonical_index = tokens.iter().position(|t| t.starts_with("--")).unwrap_or(0);
        let canonical = tokens[canonical_index].clone();
        flags.insert(canonical.clone());
        if desc.contains("alias for") || desc.contains("same as") {
            alias_desc.insert(canonical.clone(), desc.to_string());
        }
        for (i, alias) in tokens.iter().enumerate() {
            if i == canonical_index {
                continue;
            }
            flags.insert(alias.clone());
            aliases.insert(alias.clone(), canonical.clone());
        }
    }

    (flags, aliases, alias_desc)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rsync_help = Command::new("rsync").arg("--help").output()?;
    let rsync_help_str = String::from_utf8(rsync_help.stdout)?;
    let (rsync_flags, rsync_aliases, rsync_alias_desc) = parse_help(&rsync_help_str);

    let rsync_rs_help = Command::new("cargo")
        .args(["run", "--quiet", "--bin", "rsync-rs", "--", "--help"])
        .output()?;
    let rsync_rs_help_str = String::from_utf8(rsync_rs_help.stdout)?;
    let (rsync_rs_flags, rsync_rs_aliases, _rsync_rs_alias_desc) = parse_help(&rsync_rs_help_str);

    let error_notes: HashMap<&str, &str> = [
    ]
    .into_iter()
    .collect();

    let ignored_flags: BTreeSet<&str> = BTreeSet::new();

    let mut entries = Vec::new();
    for flag in rsync_flags.iter() {
        let mut status = if rsync_rs_flags.contains(flag) {
            if error_notes.contains_key(flag.as_str()) {
                "Error"
            } else if ignored_flags.contains(flag.as_str()) {
                "Ignored"
            } else if rsync_rs_aliases.contains_key(flag) {
                "Alias"
            } else {
                "Supported"
            }
        } else {
            "Error"
        };
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

    Ok(())
}
