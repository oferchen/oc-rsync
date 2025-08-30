// fuzz/fuzz_targets/filters_dir_merge_fuzz.rs
#![no_main]
use filters::{parse, Matcher};
use fuzz::helpers;
use libfuzzer_sys::fuzz_target;
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

fuzz_target!(|data: &[u8]| {
    if let Some(s) = helpers::as_str(data) {
        let parts: Vec<&str> = s.splitn(2, '\n').collect();
        let root_rules = parts.get(0).copied().unwrap_or("");
        let sub_rules = parts.get(1).copied().unwrap_or("");
        if let Ok(tmp) = tempdir() {
            let root = tmp.path();
            fs::write(root.join(".rsync-filter"), root_rules).ok();
            let sub = root.join("sub");
            fs::create_dir_all(&sub).ok();
            fs::write(sub.join(".rsync-filter"), sub_rules).ok();
            let mut v = HashSet::new();
            if let Ok(rules) = parse(": /.rsync-filter\n- .rsync-filter\n", &mut v, 0) {
                let matcher = Matcher::new(rules).with_root(root);
                let _ = matcher.is_included("sub/file");
            }
        }
    }
});
