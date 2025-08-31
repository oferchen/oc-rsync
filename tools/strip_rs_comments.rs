// tools/strip_rs_comments.rs
use std::env;
use std::fs;
use std::path::Path;

use rustc_lexer::{tokenize, TokenKind};

fn strip_comments(src: &str) -> (String, bool) {
    let mut out = String::with_capacity(src.len());
    let mut pos = 0;
    let tokens = tokenize(src);
    let mut keep_first_comment = true;
    let mut has_doc_comment = false;
    for token in tokens {
        let text = &src[pos..pos + token.len];
        match token.kind {
            TokenKind::LineComment | TokenKind::BlockComment { .. } => {
                if text.starts_with("///") {
                    has_doc_comment = true;
                } else if keep_first_comment {
                    out.push_str(text);
                }
            }
            _ => out.push_str(text),
        }
        if text.contains('\n') {
            keep_first_comment = false;
        }
        pos += token.len;
    }
    (out, has_doc_comment)
}

fn process_file(path: &Path, check: bool) -> Result<bool, Box<dyn std::error::Error>> {
    let orig = fs::read_to_string(path)?;
    let (stripped, has_doc) = strip_comments(&orig);
    if check {
        if has_doc {
            eprintln!("{}: contains doc comments", path.display());
            return Ok(false);
        }
        if orig != stripped {
            eprintln!("{}: contains disallowed comments", path.display());
            return Ok(false);
        }
    } else if has_doc || orig != stripped {
        fs::write(path, stripped)?;
    }
    Ok(true)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut check = false;
    let mut paths = Vec::new();
    for arg in env::args().skip(1) {
        if arg == "--check" {
            check = true;
        } else {
            paths.push(arg);
        }
    }
    let mut success = true;
    for path in paths {
        let path = Path::new(&path);
        let ok = process_file(path, check)?;
        if !ok {
            success = false;
        }
    }
    if check && !success {
        std::process::exit(1);
    }
    Ok(())
}
