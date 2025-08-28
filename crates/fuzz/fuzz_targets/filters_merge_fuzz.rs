#![no_main]
use filters::{parse, Matcher};
use fuzz::helpers;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some(s) = helpers::as_str(data) {
        let parts: Vec<&str> = s.splitn(2, '\n').collect();
        let first = parts.get(0).copied().unwrap_or("");
        let second = parts.get(1).copied().unwrap_or("");
        if let (Ok(r1), Ok(r2)) = (parse(first), parse(second)) {
            let mut merged = Matcher::new(r1.clone());
            merged.merge(r2.clone());
            let combined = format!("{}\n{}", first, second);
            if let Ok(all) = parse(&combined) {
                let m2 = Matcher::new(all);
                for path in ["a", "a.txt", "dir/file", "dir/sub/file.txt"] {
                    let _ = assert_eq!(merged.is_included(path), m2.is_included(path));
                }
            }
        }
    }
});
