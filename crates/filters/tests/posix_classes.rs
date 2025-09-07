// crates/filters/tests/posix_classes.rs
use filters::{Matcher, parse};
use std::collections::HashSet;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn sanitize(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ((ch as u32) < 0x20 && ch != '\t') || (ch as u32) == 0x7F {
            out.push('\\');
            out.push('#');
            out.push_str(&format!("{:03o}", ch as u32));
        } else {
            out.push(ch);
        }
    }
    out
}

fn parity(class: &str, cases: &[(&str, bool)]) {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("src");
    fs::create_dir_all(&root).unwrap();
    for (name, _) in cases {
        fs::write(root.join(name), "").unwrap();
    }
    let inc = format!("+ file[[:{}:]]name", class);
    let exc = "- *";
    let rules_src = format!("{inc}\n{exc}\n");
    let mut visited = HashSet::new();
    let rules = parse(&rules_src, &mut visited, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(&root);

    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    let root_arg = format!("{}/", root.display());
    let output = Command::new("rsync")
        .arg("-r")
        .arg("-n")
        .arg("--out-format=%n")
        .arg("-FF")
        .arg("-f")
        .arg(&inc)
        .arg("-f")
        .arg(&exc)
        .arg(&root_arg)
        .arg(&dest)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let rsync_included: Vec<String> = stdout.lines().map(|l| l.to_string()).collect();

    for (name, should_match) in cases {
        let ours = matcher.is_included(name).unwrap();
        assert_eq!(ours, *should_match, "class {class} file {:?}", name);
        let theirs = rsync_included.contains(&sanitize(name));
        assert_eq!(ours, theirs, "parity for class {class} file {:?}", name);
    }
}

#[test]
fn space_class_parity() {
    parity(
        "space",
        &[
            ("file name", true),
            ("file\tname", true),
            ("file\nname", true),
            ("filename", false),
        ],
    );
}

#[test]
fn blank_class_parity() {
    parity(
        "blank",
        &[
            ("file name", true),
            ("file\tname", true),
            ("file\nname", false),
            ("filename", false),
        ],
    );
}

#[test]
fn punct_class_parity() {
    parity(
        "punct",
        &[
            ("file-name", true),
            ("file.name", true),
            ("filename", false),
            ("file name", false),
        ],
    );
}

#[test]
fn cntrl_class_parity() {
    parity("cntrl", &[("file\u{1}name", true), ("filename", false)]);
}

#[test]
fn graph_class_parity() {
    parity(
        "graph",
        &[
            ("file-name", true),
            ("file1name", true),
            ("file name", false),
        ],
    );
}

#[test]
fn print_class_parity() {
    parity(
        "print",
        &[
            ("file-name", true),
            ("file name", true),
            ("file\u{1}name", false),
        ],
    );
}
