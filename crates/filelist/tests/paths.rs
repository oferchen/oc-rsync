// crates/filelist/tests/paths.rs
use filelist::{Decoder, Encoder, Entry};
use tempfile::tempdir;

fn tag_positions(bytes: &[u8]) -> (usize, usize) {
    let suffix_len = bytes[1] as usize;
    let mut idx = 2 + suffix_len;
    let uid_pos = idx;
    if bytes[idx] == 0xFF {
        idx += 5; // tag + 4-byte id
    } else {
        idx += 1;
    }
    let gid_pos = idx;
    (uid_pos, gid_pos)
}

#[test]
fn encodes_paths_and_reuses_id_tables() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path();
    let entry1 = Entry {
        path: dir
            .join("file1.txt")
            .to_string_lossy()
            .into_owned()
            .into_bytes(),
        uid: 1000,
        gid: 1000,
        hardlink: None,
        xattrs: Vec::new(),
        acl: Vec::new(),
        default_acl: Vec::new(),
    };
    let entry2 = Entry {
        path: dir
            .join("file2.txt")
            .to_string_lossy()
            .into_owned()
            .into_bytes(),
        uid: 1000,
        gid: 1000,
        hardlink: None,
        xattrs: Vec::new(),
        acl: Vec::new(),
        default_acl: Vec::new(),
    };

    let mut enc = Encoder::new();
    let bytes1 = enc.encode_entry(&entry1);
    let bytes2 = enc.encode_entry(&entry2);

    let expected_common = entry1
        .path
        .iter()
        .zip(entry2.path.iter())
        .take_while(|(a, b)| a == b)
        .count();
    assert_eq!(bytes2[0] as usize, expected_common);

    let (uid_pos1, gid_pos1) = tag_positions(&bytes1);
    let (uid_pos2, gid_pos2) = tag_positions(&bytes2);
    assert_eq!(bytes1[uid_pos1], 0xFF);
    assert_eq!(bytes1[gid_pos1], 0xFF);
    assert_eq!(bytes2[uid_pos2], 0);
    assert_eq!(bytes2[gid_pos2], 0);

    let mut dec = Decoder::new();
    let decoded1 = dec.decode_entry(&bytes1).unwrap();
    let decoded2 = dec.decode_entry(&bytes2).unwrap();
    assert_eq!(decoded1, entry1);
    assert_eq!(decoded2, entry2);
}
