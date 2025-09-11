// crates/filelist/tests/acl.rs
use filelist::{Decoder, Encoder, Entry};

#[test]
fn roundtrip_acl_entries() {
    let entry = Entry {
        path: b"file".to_vec(),
        uid: 1,
        gid: 2,
        hardlink: None,
        xattrs: Vec::new(),
        acl: vec![1, 0, 0, 0, 0, 7, 0, 0, 0],
        default_acl: vec![1, 0, 0, 0, 0, 7, 0, 0, 0],
    };
    let mut enc = Encoder::new();
    let payload = enc.encode_entry(&entry);
    let mut dec = Decoder::new();
    let decoded = dec.decode_entry(&payload).unwrap();
    assert_eq!(decoded, entry);
}

#[test]
fn roundtrip_root_default_acl() {
    let entry = Entry {
        path: Vec::new(),
        uid: 0,
        gid: 0,
        hardlink: None,
        xattrs: Vec::new(),
        acl: Vec::new(),
        default_acl: vec![1, 0, 0, 0, 0, 7, 0, 0, 0],
    };
    let mut enc = Encoder::new();
    let payload = enc.encode_entry(&entry);
    let mut dec = Decoder::new();
    let decoded = dec.decode_entry(&payload).unwrap();
    assert_eq!(decoded, entry);
}
