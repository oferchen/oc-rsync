// xtask/src/bin/comment_lint.rs
use rustc_lexer::{TokenKind, tokenize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn check_file(path: &Path, root: &Path) -> bool {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let Ok(content) = fs::read_to_string(&abs) else {
        return false;
    };
    let rel = abs.strip_prefix(root).unwrap_or(&abs);
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    let first = content.lines().next().unwrap_or("");
    let header = format!("// {}", rel_str);
    if first.trim_end() != header {
        eprintln!("{}: incorrect header", rel_str);
        return false;
    }
    let mut pos = 0;
    let mut first_line = true;
    for token in tokenize(&content) {
        let text = &content[pos..pos + token.len];
        if matches!(
            token.kind,
            TokenKind::LineComment | TokenKind::BlockComment { .. }
        ) {
            if first_line {
                if text.starts_with("///") {
                    eprintln!("{}: doc comment", rel_str);
                    return false;
                }
            } else if !text.starts_with("///") {
                eprintln!("{}: additional comments", rel_str);
                return false;
            }
        }
        if text.contains('\n') {
            first_line = false;
        }
        pos += token.len;
    }
    true
}

fn main() {
    let root = env::current_dir().unwrap();
    let mut ok = true;
    for arg in env::args().skip(1) {
        if !check_file(PathBuf::from(arg).as_path(), &root) {
            ok = false;
        }
    }
    if !ok {
        std::process::exit(1);
    }
}
