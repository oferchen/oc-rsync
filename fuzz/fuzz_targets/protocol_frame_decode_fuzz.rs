// fuzz/fuzz_targets/protocol_frame_decode_fuzz.rs
#![no_main]
use fuzz::helpers;
use libfuzzer_sys::fuzz_target;
use protocol::Frame;

fuzz_target!(|data: &[u8]| {
    let mut reader = helpers::cursor(data);
    let _ = Frame::decode(&mut reader);
});
