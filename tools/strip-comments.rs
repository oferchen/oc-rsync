// tools/strip-comments.rs
use std::env;
use std::fs;
use std::path::Path;

use rustc_lexer::{tokenize, TokenKind};

fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut pos = 0;
    let tokens = tokenize(src);
    let mut keep_first_comment = true;
    for token in tokens {
        let text = &src[pos..pos + token.len];
        match token.kind {
            TokenKind::LineComment | TokenKind::BlockComment { .. } => {
                if keep_first_comment {
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
    out
}

fn process_file(path: &Path, check: bool) -> Result<bool, Box<dyn std::error::Error>> {
    let orig = fs::read_to_string(path)?;
    let stripped = strip_comments(&orig);
    if check {
        if orig != stripped {
            eprintln!("{}: contains disallowed comments", path.display());
            return Ok(false);
        }
    } else if orig != stripped {
        fs::write(path, stripped)?;
    }
    Ok(true)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mut check = false;
    let mut paths = Vec::new();
    while let Some(arg) = args.next() {
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
