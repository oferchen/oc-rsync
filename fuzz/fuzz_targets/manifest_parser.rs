// fuzz/fuzz_targets/manifest_parser.rs
#![no_main]
use engine::cdc::Manifest;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = Manifest::parse(data);
});
