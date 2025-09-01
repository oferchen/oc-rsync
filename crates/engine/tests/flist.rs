// crates/engine/tests/flist.rs
use engine::flist;
use filelist::Entry;

#[test]
fn roundtrip() {
    let entries = vec![
        Entry {
            path: "a".into(),
            uid: 1,
            gid: 2,
            group: None,
        },
        Entry {
            path: "a/b".into(),
            uid: 1,
            gid: 3,
            group: None,
        },
        Entry {
            path: "c".into(),
            uid: 4,
            gid: 3,
            group: None,
        },
    ];
    let payloads = flist::encode(&entries);
    let decoded = flist::decode(&payloads).unwrap();
    assert_eq!(decoded, entries);
}
