// crates/filelist/src/lib.rs

#![doc = include_str!("../../../docs/crates/filelist/lib.md")]

pub mod decoder;
pub mod encoder;
pub mod entry;

pub use decoder::{DecodeError, Decoder};
pub use encoder::Encoder;
pub use entry::{Entry, InodeEntry, group_by_inode};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_paths_and_ids() {
        let entries = vec![
            Entry {
                path: b"dir/file1".to_vec(),
                uid: 1000,
                gid: 1000,
                hardlink: None,
                xattrs: vec![(b"user.test".to_vec(), b"val".to_vec())],
                acl: vec![1, 0, 0, 0, 0, 7, 0, 0, 0],
                default_acl: Vec::new(),
            },
            Entry {
                path: b"dir/file2".to_vec(),
                uid: 1000,
                gid: 1001,
                hardlink: Some(2000),
                xattrs: Vec::new(),
                acl: Vec::new(),
                default_acl: vec![1, 0, 0, 0, 0, 7, 0, 0, 0],
            },
            Entry {
                path: b"other".to_vec(),
                uid: 1002,
                gid: 1001,
                hardlink: Some(2000),
                xattrs: Vec::new(),
                acl: Vec::new(),
                default_acl: Vec::new(),
            },
        ];
        let mut enc = Encoder::new();
        let mut dec = Decoder::new();
        for e in entries {
            let bytes = enc.encode_entry(&e);
            let d = dec.decode_entry(&bytes).unwrap();
            assert_eq!(d, e);
        }
    }

    #[test]
    fn path_delta_encode_decode() {
        let e1 = Entry {
            path: b"dir/file1".to_vec(),
            uid: 0,
            gid: 0,
            hardlink: None,
            xattrs: Vec::new(),
            acl: Vec::new(),
            default_acl: Vec::new(),
        };
        let e2 = Entry {
            path: b"dir/file2".to_vec(),
            uid: 0,
            gid: 0,
            hardlink: None,
            xattrs: Vec::new(),
            acl: Vec::new(),
            default_acl: Vec::new(),
        };
        let mut enc = Encoder::new();
        let mut dec = Decoder::new();

        let b1 = enc.encode_entry(&e1);
        dec.decode_entry(&b1).unwrap();

        let b2 = enc.encode_entry(&e2);
        assert_eq!(&b2[..3], &[8, 1, b'2']);
        let d2 = dec.decode_entry(&b2).unwrap();
        assert_eq!(d2.path, e2.path);
    }

    #[test]
    fn decode_errors() {
        let mut dec = Decoder::new();
        assert_eq!(dec.decode_entry(&[0]).unwrap_err(), DecodeError::ShortInput);

        let mut dec = Decoder::new();
        let bytes = vec![0, 1, b'a', 0];
        assert_eq!(
            dec.decode_entry(&bytes).unwrap_err(),
            DecodeError::BadUid(0)
        );

        let mut dec = Decoder::new();
        let bytes = vec![0, 1, b'a', 0xFF, 232, 3, 0, 0, 0];
        assert_eq!(
            dec.decode_entry(&bytes).unwrap_err(),
            DecodeError::BadGid(0)
        );
    }
}
