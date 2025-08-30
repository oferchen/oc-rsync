// fuzz/fuzz_targets/dirent_parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::fs;
use walk::Entry;

fuzz_target!(|data: &[u8]| {
    let mut i = 0;
    let mut state = String::new();
    let file_type = fs::metadata(".").map(|m| m.file_type()).unwrap();
    while i + 2 <= data.len() {
        let prefix_len = data[i] as usize;
        let suffix_len = data[i + 1] as usize;
        i += 2;
        if i + suffix_len > data.len() {
            break;
        }
        if let Ok(suffix) = std::str::from_utf8(&data[i..i + suffix_len]) {
            let entry = Entry {
                prefix_len,
                suffix: suffix.to_string(),
                file_type,
                uid: 0,
                gid: 0,
                dev: 0,
            };
            let _ = entry.apply(&mut state);
        }
        i += suffix_len;
    }
});
