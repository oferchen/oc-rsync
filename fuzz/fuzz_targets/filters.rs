// fuzz/fuzz_targets/filters.rs
#![no_main]
use filters::{parse, Matcher};
use fuzz::helpers;
use libfuzzer_sys::fuzz_target;
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

fuzz_target!(|data: &[u8]| {
    if let Some(s) = helpers::as_str(data) {
        let mut v = HashSet::new();
        if let Ok(rules) = parse(s, &mut v, 0) {
            let tmp = tempdir().unwrap();
            fs::write(tmp.path().join("existing"), b"").ok();
            let matcher = Matcher::new(rules).with_root(tmp.path());
            let _ = matcher.is_included("existing");
            let _ = matcher.is_included("missing");
        }
    }
});
