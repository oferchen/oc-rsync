// fuzz/fuzz_targets/filter_parser.rs
#![no_main]
use filters::parse;
use fuzz::helpers;
use libfuzzer_sys::fuzz_target;
use std::collections::HashSet;

fuzz_target!(|data: &[u8]| {
    if let Some(s) = helpers::as_str(data) {
        let mut v = HashSet::new();
        let _ = parse(s, &mut v, 0);
        let mut v = HashSet::new();
        let merged = format!(".w {}", s);
        let _ = parse(&merged, &mut v, 0);
        let mut v = HashSet::new();
        let dirmerged = format!(":+ {}", s);
        let _ = parse(&dirmerged, &mut v, 0);
        let mut v = HashSet::new();
        let dirmerged2 = format!(":- {}", s);
        let _ = parse(&dirmerged2, &mut v, 0);
    }
});
