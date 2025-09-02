// crates/engine/tests/flist.rs
use encoding_rs::Encoding;
use engine::flist;
use filelist::Entry;
use protocol::CharsetConv;

#[test]
fn roundtrip() {
    let entries = vec![
        Entry {
            path: b"a".to_vec(),
            uid: 1,
            gid: 2,
            group: None,
            xattrs: vec![(b"user.test".to_vec(), b"1".to_vec())],
            acl: vec![1, 0, 0, 0, 0, 7, 0, 0, 0],
            default_acl: Vec::new(),
        },
        Entry {
            path: b"a/b".to_vec(),
            uid: 1,
            gid: 3,
            group: None,
            xattrs: Vec::new(),
            acl: Vec::new(),
            default_acl: vec![1, 0, 0, 0, 0, 7, 0, 0, 0],
        },
        Entry {
            path: b"c".to_vec(),
            uid: 4,
            gid: 3,
            group: None,
            xattrs: Vec::new(),
            acl: Vec::new(),
            default_acl: Vec::new(),
        },
    ];
    let payloads = flist::encode(&entries, None);
    let decoded = flist::decode(&payloads, None).unwrap();
    assert_eq!(decoded, entries);
}

#[test]
fn group_id_roundtrip() {
    let entries = vec![
        Entry {
            path: b"a".to_vec(),
            uid: 1,
            gid: 2,
            group: Some(42),
            xattrs: Vec::new(),
            acl: Vec::new(),
            default_acl: Vec::new(),
        },
        Entry {
            path: b"a/b".to_vec(),
            uid: 1,
            gid: 3,
            group: Some(42),
            xattrs: Vec::new(),
            acl: Vec::new(),
            default_acl: Vec::new(),
        },
        Entry {
            path: b"c".to_vec(),
            uid: 4,
            gid: 3,
            group: Some(99),
            xattrs: Vec::new(),
            acl: Vec::new(),
            default_acl: Vec::new(),
        },
    ];
    let payloads = flist::encode(&entries, None);
    let decoded = flist::decode(&payloads, None).unwrap();
    assert_eq!(decoded, entries);
}

#[test]
fn iconv_roundtrip() {
    let cv = CharsetConv::new(
        Encoding::for_label(b"latin1").unwrap(),
        Encoding::for_label(b"utf-8").unwrap(),
    );
    let entries = vec![Entry {
        path: "Grüße".as_bytes().to_vec(),
        uid: 0,
        gid: 0,
        group: None,
        xattrs: Vec::new(),
        acl: Vec::new(),
        default_acl: Vec::new(),
    }];
    let payloads = flist::encode(&entries, Some(&cv));
    let decoded = flist::decode(&payloads, Some(&cv)).unwrap();
    assert_eq!(decoded, entries);
}

#[test]
fn iconv_non_utf8_local_roundtrip() {
    let cv = CharsetConv::new(
        Encoding::for_label(b"utf-8").unwrap(),
        Encoding::for_label(b"latin1").unwrap(),
    );
    let entries = vec![Entry {
        path: b"f\xF8o".to_vec(),
        uid: 0,
        gid: 0,
        group: None,
        xattrs: Vec::new(),
        acl: Vec::new(),
        default_acl: Vec::new(),
    }];
    let payloads = flist::encode(&entries, Some(&cv));
    let decoded = flist::decode(&payloads, Some(&cv)).unwrap();
    assert_eq!(decoded, entries);
}
