// crates/filters/src/parser/parse/token.rs

use crate::perdir::PerDir;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeModifier {
    IncludeOnly,
    ExcludeOnly,
    NoInherit,
    WordSplit,
    CvsMode,
}

impl MergeModifier {
    pub fn apply(self, pd: &mut PerDir) {
        match self {
            MergeModifier::IncludeOnly => pd.sign = Some('+'),
            MergeModifier::ExcludeOnly => pd.sign = Some('-'),
            MergeModifier::NoInherit => pd.inherit = false,
            MergeModifier::WordSplit => pd.word_split = true,
            MergeModifier::CvsMode => pd.cvs = true,
        }
    }
}

pub enum RuleKind {
    Include,
    Exclude,
    Protect,
}

pub fn split_mods<'a>(s: &'a str, allowed: &str) -> (&'a str, &'a str) {
    if let Some(ch) = s.chars().next() {
        if ch.is_whitespace() {
            let rest = &s[ch.len_utf8()..];
            return ("", rest);
        }
    } else {
        return ("", "");
    }
    let mut idx = 0;
    let bytes = s.as_bytes();
    if bytes.first() == Some(&b',') {
        idx += 1;
    }
    while let Some(&c) = bytes.get(idx) {
        if allowed.as_bytes().contains(&c) {
            idx += 1;
        } else {
            break;
        }
    }
    let rest = &s[idx..];
    let rest = if rest.starts_with(' ') || rest.starts_with('\t') {
        &rest[1..]
    } else {
        rest
    };
    (&s[..idx], rest)
}
