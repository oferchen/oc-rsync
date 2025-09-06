// crates/protocol/tests/charset_conv.rs
use encoding_rs::Encoding;
use protocol::CharsetConv;
use std::borrow::Cow;

#[test]
fn round_trip_between_encodings() {
    let cv = CharsetConv::new(
        Encoding::for_label(b"iso-8859-1").unwrap(),
        Encoding::for_label(b"utf-8").unwrap(),
    );
    let text = "Grüße";
    let encoded = cv.encode_remote(text);
    assert_eq!(encoded.as_ref(), &[0x47, 0x72, 0xFC, 0xDF, 0x65]);
    let decoded = cv.decode_remote(encoded.as_ref());
    assert_eq!(decoded, "Grüße");

    let remote = cv.to_remote(text.as_bytes());
    assert_eq!(remote.as_ref(), encoded.as_ref());
    let local = cv.to_local(remote.as_ref());
    assert_eq!(local.as_ref(), text.as_bytes());
}

#[test]
fn identity_conversions_borrow() {
    let cv = CharsetConv::new(
        Encoding::for_label(b"utf-8").unwrap(),
        Encoding::for_label(b"utf-8").unwrap(),
    );
    let s = "hello";
    let enc = cv.encode_remote(s);
    assert!(matches!(enc, Cow::Borrowed(_)));
    let dec = cv.decode_remote(s.as_bytes());
    assert!(matches!(dec, Cow::Borrowed(_)));
    let to_remote = cv.to_remote(s.as_bytes());
    assert!(matches!(to_remote, Cow::Borrowed(_)));
    let to_local = cv.to_local(s.as_bytes());
    assert!(matches!(to_local, Cow::Borrowed(_)));
}
