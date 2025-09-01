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
        },
        Entry {
            path: b"a/b".to_vec(),
            uid: 1,
            gid: 3,
            group: None,
        },
        Entry {
            path: b"c".to_vec(),
            uid: 4,
            gid: 3,
            group: None,
        },
    ];
    let payloads = flist::encode(&entries, None);
    let decoded = flist::decode(&payloads, None).unwrap();
    assert_eq!(decoded, entries);
}

#[test]
fn iconv_roundtrip() {
    let cv = CharsetConv::new(Encoding::for_label(b"latin1").unwrap());
    let entries = vec![Entry {
        path: "Grüße".as_bytes().to_vec(),
        uid: 0,
        gid: 0,
    }];
    let payloads = flist::encode(&entries, Some(&cv));
    let decoded = flist::decode(&payloads, Some(&cv)).unwrap();
    assert_eq!(decoded, entries);
}
