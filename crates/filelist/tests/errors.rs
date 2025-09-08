// crates/filelist/tests/errors.rs
use filelist::{DecodeError, Decoder};

#[test]
fn decode_error_short_input() {
    let mut dec = Decoder::new();
    let err = dec.decode_entry(&[]).unwrap_err();
    assert_eq!(err, DecodeError::ShortInput);
}

#[test]
fn decode_error_bad_uid() {
    let mut dec = Decoder::new();
    // path: common=0, len=1, suffix='a', uid index 1
    let bytes = vec![0, 1, b'a', 1];
    let err = dec.decode_entry(&bytes).unwrap_err();
    assert_eq!(err, DecodeError::BadUid(1));
}

#[test]
fn decode_error_bad_gid() {
    let mut dec = Decoder::new();
    // path: common=0, len=1, suffix='a', uid new 0, gid index 2
    let bytes = vec![0, 1, b'a', 0xFF, 0, 0, 0, 0, 2];
    let err = dec.decode_entry(&bytes).unwrap_err();
    assert_eq!(err, DecodeError::BadGid(2));
}
