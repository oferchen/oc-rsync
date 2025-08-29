#![no_main]
use filters::{parse, Matcher};
use fuzz::helpers;
use libfuzzer_sys::fuzz_target;
use std::collections::HashSet;

fuzz_target!(|data: &[u8]| {
    if let Some(s) = helpers::as_str(data) {
        let mut v = HashSet::new();
        if let Ok(rules) = parse(s, &mut v, 0) {
            let matcher = Matcher::new(rules);
            let _ = matcher.is_included("test");
        }
    }
});
