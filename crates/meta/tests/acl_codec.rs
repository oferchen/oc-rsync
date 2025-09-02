// crates/meta/tests/acl_codec.rs
#![cfg(feature = "acl")]

use meta::{decode_acl, encode_acl, Metadata, Options};
use posix_acl::{PosixACL, Qualifier, ACL_READ};
use std::fs;
use tempfile::tempdir;

fn acl_to_io(err: posix_acl::ACLError) -> std::io::Error {
    if let Some(ioe) = err.as_io_error() {
        if let Some(code) = ioe.raw_os_error() {
            std::io::Error::from_raw_os_error(code)
        } else {
            std::io::Error::new(ioe.kind(), ioe.to_string())
        }
    } else {
        std::io::Error::other(err)
    }
}

#[test]
fn encode_decode_roundtrip_acl() -> std::io::Result<()> {
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::write(&src, b"hello")?;
    fs::write(&dst, b"world")?;

    let mut acl = PosixACL::read_acl(&src).map_err(acl_to_io)?;
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&src).map_err(acl_to_io)?;

    let opts = Options {
        acl: true,
        ..Default::default()
    };
    let meta = Metadata::from_path(&src, opts.clone())?;
    let data = encode_acl(&meta.acl);
    let decoded = decode_acl(&data);
    let mut meta2 = meta.clone();
    meta2.acl = decoded;
    meta2.apply(&dst, opts.clone())?;
    let applied = Metadata::from_path(&dst, opts)?;
    assert_eq!(meta.acl, applied.acl);
    Ok(())
}
