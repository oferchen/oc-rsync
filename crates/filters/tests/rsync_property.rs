// crates/filters/tests/rsync_property.rs
use filters::{Matcher, parse};
use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn rule_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("+ *.log".to_string()),
        Just("+ *.tmp".to_string()),
        Just("- *.log".to_string()),
        Just("- *.tmp".to_string()),
        Just("P keep.tmp".to_string()),
        Just("-! */".to_string()),
        Just("+! */".to_string()),
        Just("hide *.tmp".to_string()),
        Just("show keep.log".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]
    #[test]
    fn random_rule_parity(extra in prop::collection::vec(rule_strategy(), 0..4)) {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("src");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join(".rsync-filter"), "- *.tmp\n").unwrap();
        fs::write(root.join("a.tmp"), "").unwrap();
        fs::write(root.join("keep.log"), "").unwrap();
        fs::write(root.join("sub/.rsync-filter"), "+ keep.tmp\n").unwrap();
        fs::write(root.join("sub/keep.tmp"), "").unwrap();
        fs::write(root.join("sub/other.tmp"), "").unwrap();

        let mut rules_src = String::from(": /.rsync-filter\n- .rsync-filter\n");
        for r in &extra { rules_src.push_str(r); rules_src.push('\n'); }
        let mut v = HashSet::new();
        let rules = parse(&rules_src, &mut v, 0).unwrap();
        let matcher = Matcher::new(rules).with_root(&root);

        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();
        let root_arg = format!("{}/", root.display());
        let mut args = vec!["-r", "-n", "-i", "-FF", &root_arg, dest.to_str().unwrap()];
        for r in &extra {
            args.insert(args.len()-2, "-f");
            args.insert(args.len()-2, r);
        }
        args.insert(0, "rsync");
        let output = Command::new(args[0])
            .args(&args[1..])
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut rsync_included = Vec::new();
        for line in stdout.lines() {
            if line.starts_with("sending ") || line.starts_with("sent ") || line.starts_with("total ") { continue; }
            if let Some(name) = line.split_whitespace().last() { rsync_included.push(name.to_string()); }
        }

        let files = [
            "a.tmp",
            "keep.log",
            "sub/keep.tmp",
            "sub/other.tmp",
            ".rsync-filter",
            "sub/.rsync-filter",
        ];
        for f in files {
            let ours = matcher.is_included(f).unwrap();
            let theirs = rsync_included.contains(&f.to_string());
            prop_assert_eq!(ours, theirs, "file {}", f);
        }
    }
}
