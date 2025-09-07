// crates/filters/tests/rsync_special_chars.rs
#![cfg(all(unix, feature = "interop"))]

use filters::{Matcher, parse};
use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn pattern_strategy() -> impl Strategy<Value = String> {
    let chars =
        prop::collection::vec(prop::sample::select(&['a', 'b', 'c']), 1..4).prop_map(|mut v| {
            v.sort();
            v.dedup();
            v.into_iter().collect::<String>()
        });
    (prop_oneof![Just("+"), Just("-")], any::<bool>(), chars).prop_map(|(sign, bang, chars)| {
        let mut rule = sign.to_string();
        if bang {
            rule.push('!');
        }
        rule.push_str(" dir/[!");
        rule.push_str(&chars);
        rule.push(']');
        rule
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    #[ignore = "requires rsync"]
    fn special_chars_parity(rule in pattern_strategy()) {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("src");
        fs::create_dir_all(root.join("dir")).unwrap();
        fs::write(root.join("dir/a"), "").unwrap();
        fs::write(root.join("dir/b"), "").unwrap();
        fs::write(root.join("dir/c"), "").unwrap();

        let mut visited = HashSet::new();
        let rules_src = format!("{rule}\n");
        let rules = parse(&rules_src, &mut visited, 0).unwrap();
        let matcher = Matcher::new(rules).with_root(&root);

        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();
        let root_arg = format!("{}/", root.display());
        let output = Command::new("rsync")
            .arg("-r")
            .arg("-n")
            .arg("-i")
            .arg("-FF")
            .arg("-f")
            .arg(&rule)
            .arg(&root_arg)
            .arg(&dest)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut rsync_included = Vec::new();
        for line in stdout.lines() {
            if line.starts_with("sending ") || line.starts_with("sent ") || line.starts_with("total ") { continue; }
            if let Some(name) = line.split_whitespace().last() { rsync_included.push(name.to_string()); }
        }

        let paths = ["dir", "dir/a", "dir/b", "dir/c"];
        for p in paths {
            let ours = matcher.is_included(p).unwrap();
            let candidate = if p == "dir" { format!("{p}/") } else { p.to_string() };
            let theirs = rsync_included.contains(&candidate);
            prop_assert_eq!(ours, theirs, "rule {} path {}", rule, p);
        }
    }
}
