#![no_main]
use fuzz::helpers;
use libfuzzer_sys::fuzz_target;
use std::collections::HashSet;
use filters::parse;

fuzz_target!(|data: &[u8]| {
    if let Some(s) = helpers::as_str(data) {
        let mut v = HashSet::new();
        let _ = parse(s, &mut v, 0);
    }
});
