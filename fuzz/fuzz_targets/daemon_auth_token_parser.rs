// fuzz/fuzz_targets/daemon_auth_token_parser.rs
#![no_main]
use daemon::parse_auth_token;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some(pos) = data.iter().position(|b| *b == b'\n') {
        if let (Ok(token), Ok(contents)) = (
            std::str::from_utf8(&data[..pos]),
            std::str::from_utf8(&data[pos + 1..]),
        ) {
            let _ = parse_auth_token(token.trim(), contents);
        }
    }
});
