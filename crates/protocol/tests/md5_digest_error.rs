// crates/protocol/tests/md5_digest_error.rs
use std::convert::TryInto;
use std::io;

#[test]
fn md5_digest_wrong_length_errors() {
    let digest = vec![0u8; 15];
    let res: io::Result<[u8; 16]> = digest
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "MD5 digests must be 16 bytes"));
    assert!(res.is_err());
}
