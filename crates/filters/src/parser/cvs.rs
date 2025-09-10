// crates/filters/src/parser/cvs.rs

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;

use super::{ParseError, parse::parse};
use crate::rule::Rule;

pub const CVS_DEFAULTS: &[&str] = &[
    "RCS",
    "SCCS",
    "CVS",
    "CVS.adm",
    "RCSLOG",
    "cvslog.*",
    "tags",
    "TAGS",
    ".make.state",
    ".nse_depinfo",
    "*~",
    "#*",
    ".#*",
    ",*",
    "_$*",
    "*$",
    "*.old",
    "*.bak",
    "*.BAK",
    "*.orig",
    "*.rej",
    ".del-*",
    "*.a",
    "*.olb",
    "*.o",
    "*.obj",
    "*.so",
    "*.exe",
    "*.Z",
    "*.elc",
    "*.ln",
    "core",
    ".svn/",
    ".git/",
    ".hg/",
    ".bzr/",
];

pub fn default_cvs_rules() -> Result<Vec<Rule>, ParseError> {
    fn append_rule(buf: &mut String, tok: &str) {
        if tok.is_empty() {
            return;
        }
        let mut chars = tok.chars();
        if let Some(first) = chars.next() {
            if matches!(first, '+' | '-' | 'P' | 'p' | 'S' | 'H' | 'R' | '!') {
                let rest = chars.as_str();
                let mods_len = rest.chars().take_while(|c| "!/Csrpx".contains(*c)).count();
                let (mods, pat) = rest.split_at(mods_len);
                let mut mods = mods.to_string();
                if !mods.contains('p') {
                    mods.push('p');
                }
                if pat.is_empty() {
                    buf.push(first);
                    buf.push_str(&mods);
                } else {
                    buf.push(first);
                    buf.push_str(&mods);
                    buf.push(' ');
                    buf.push_str(pat);
                }
            } else {
                buf.push_str("-p ");
                buf.push_str(tok);
            }
            buf.push('\n');
        }
    }

    let mut buf = String::new();
    for pat in CVS_DEFAULTS {
        append_rule(&mut buf, pat);
    }
    let mut rules = parse(&buf, &mut HashSet::new(), 0)?;
    for rule in &mut rules {
        match rule {
            Rule::Include(d) | Rule::Exclude(d) | Rule::Protect(d) | Rule::ImpliedDir(d) => {
                d.flags.perishable = true
            }
            Rule::DirMerge(pd) => pd.flags.perishable = true,
            _ => {}
        }
    }

    if let Ok(content) = env::var("CVSIGNORE") {
        let mut buf = String::new();
        for tok in content.split_ascii_whitespace() {
            append_rule(&mut buf, tok);
        }
        let mut v = parse(&buf, &mut HashSet::new(), 0)?;
        for rule in &mut v {
            match rule {
                Rule::Include(d) | Rule::Exclude(d) | Rule::Protect(d) | Rule::ImpliedDir(d) => {
                    d.flags.perishable = true;
                }
                Rule::DirMerge(pd) => pd.flags.perishable = true,
                _ => {}
            }
        }
        rules.append(&mut v);
    }

    if let Ok(home) = env::var("HOME") {
        let path = Path::new(&home).join(".cvsignore");
        if let Ok(content) = fs::read_to_string(path) {
            let mut buf = String::new();
            for tok in content.split_whitespace() {
                append_rule(&mut buf, tok);
            }
            let mut v = parse(&buf, &mut HashSet::new(), 0)?;
            for rule in &mut v {
                match rule {
                    Rule::Include(d)
                    | Rule::Exclude(d)
                    | Rule::Protect(d)
                    | Rule::ImpliedDir(d) => {
                        d.flags.perishable = true;
                    }
                    Rule::DirMerge(pd) => pd.flags.perishable = true,
                    _ => {}
                }
            }
            rules.append(&mut v);
        }
    }
    Ok(rules)
}
