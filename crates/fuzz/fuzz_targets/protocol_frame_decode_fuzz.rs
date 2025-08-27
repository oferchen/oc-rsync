#![no_main]
use libfuzzer_sys::fuzz_target;
use protocol::protocol::Frame;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    let _ = Frame::decode(&mut Cursor::new(data));
});
