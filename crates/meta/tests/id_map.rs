// crates/meta/tests/id_map.rs
use std::fs;
use std::sync::Arc;

use meta::{Metadata, Options};
use nix::unistd::{Gid, Uid, chown};
use tempfile::tempdir;

#[test]
fn uid_gid_mapping() -> std::io::Result<()> {
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::write(&src, b"hello")?;
    fs::write(&dst, b"world")?;

    if !Uid::effective().is_root() {
        eprintln!("skipping uid_gid_mapping: requires root");
        return Ok(());
    }

    chown(&src, Some(Uid::from_raw(1)), Some(Gid::from_raw(1)))?;
    chown(&dst, Some(Uid::from_raw(0)), Some(Gid::from_raw(0)))?;

    let meta = Metadata::from_path(
        &src,
        Options {
            owner: true,
            group: true,
            ..Default::default()
        },
    )?;

    let opts = Options {
        owner: true,
        group: true,
        uid_map: Some(Arc::new(|uid| if uid == 1 { 2 } else { uid })),
        gid_map: Some(Arc::new(|gid| if gid == 1 { 2 } else { gid })),
        ..Default::default()
    };
    meta.apply(&dst, opts.clone())?;
    let applied = Metadata::from_path(&dst, opts)?;
    assert_eq!(applied.uid, 2);
    assert_eq!(applied.gid, 2);
    Ok(())
}
