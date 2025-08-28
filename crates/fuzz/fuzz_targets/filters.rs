#![no_main]
use filters::{parse, Matcher};
use fuzz::helpers;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some(s) = helpers::as_str(data) {
        if let Ok(rules) = parse(s) {
            let matcher = Matcher::new(rules);
            let _ = matcher.is_included("test");
        }
    }
});
