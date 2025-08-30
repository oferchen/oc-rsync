// fuzz/fuzz_targets/filters_merge_fuzz.rs
#![no_main]
use filters::{parse, Matcher};
use std::collections::HashSet;
use fuzz::helpers;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some(s) = helpers::as_str(data) {
        let parts: Vec<&str> = s.splitn(2, '\n').collect();
        let first = parts.get(0).copied().unwrap_or("");
        let second = parts.get(1).copied().unwrap_or("");
        let (mut v1, mut v2, mut v3) = (HashSet::new(), HashSet::new(), HashSet::new());
        if let (Ok(r1), Ok(r2)) = (parse(first, &mut v1, 0), parse(second, &mut v2, 0)) {
            let mut merged = Matcher::new(r1.clone());
            merged.merge(r2.clone());
            let combined = format!("{}\n{}", first, second);
            if let Ok(all) = parse(&combined, &mut v3, 0) {
                let m2 = Matcher::new(all);
                for path in ["a", "a.txt", "dir/file", "dir/sub/file.txt"] {
                    let _ = assert_eq!(merged.is_included(path), m2.is_included(path));
                }
            }
        }
    }
});
