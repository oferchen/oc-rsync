#![no_main]
use fuzz::helpers;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some(s) = helpers::as_str(data) {
        let _ = filters::parse(s);
    }
});
