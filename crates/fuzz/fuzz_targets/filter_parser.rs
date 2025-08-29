#![no_main]
use filters::parse;
use fuzz::helpers;
use libfuzzer_sys::fuzz_target;
use std::collections::HashSet;

fuzz_target!(|data: &[u8]| {
    if let Some(s) = helpers::as_str(data) {
        let mut v = HashSet::new();
        let _ = parse(s, &mut v, 0);
    }
});
