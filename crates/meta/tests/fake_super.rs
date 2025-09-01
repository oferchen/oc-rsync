// crates/meta/tests/fake_super.rs
#[cfg(all(unix, feature = "xattr"))]
use meta::{Metadata, Options};
#[cfg(all(unix, feature = "xattr"))]
use std::fs;
#[cfg(all(unix, feature = "xattr"))]
use tempfile::tempdir;

#[cfg(all(unix, feature = "xattr"))]
use nix::unistd::Uid;

#[cfg(all(unix, feature = "xattr"))]
#[test]
fn fake_super_roundtrip() -> std::io::Result<()> {
    if Uid::effective().is_root() {
        eprintln!("skipping test as root");
        return Ok(());
    }
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::write(&src, b"hello")?;
    fs::write(&dst, b"world")?;
    #[cfg(feature = "xattr")]
    {
        xattr::set(&src, "user.rsync.uid", b"0")?;
        xattr::set(&src, "user.rsync.gid", b"0")?;
        xattr::set(&src, "user.rsync.mode", b"4755")?;
    }
    let opts = Options {
        owner: true,
        group: true,
        perms: true,
        fake_super: true,
        xattrs: true,
        ..Default::default()
    };
    let meta = Metadata::from_path(&src, opts.clone())?;
    meta.apply(&dst, opts.clone())?;
    #[cfg(feature = "xattr")]
    meta::store_fake_super(&dst, meta.uid, meta.gid, meta.mode);
    let applied = Metadata::from_path(&dst, opts)?;
    assert_eq!(meta.uid, applied.uid);
    assert_eq!(meta.gid, applied.gid);
    assert_eq!(meta.mode, applied.mode);
    Ok(())
}
