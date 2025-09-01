// crates/meta/tests/chmod.rs
use std::fs;
use std::os::unix::fs::PermissionsExt;

use meta::{normalize_mode, parse_chmod, Metadata, Options};
use nix::sys::stat::{fchmodat, FchmodatFlags, Mode};
use tempfile::tempdir;

#[test]
fn chmod_numeric_mode_normalized() -> std::io::Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("file");
    fs::write(&path, b"test")?;

    fchmodat(
        None,
        &path,
        Mode::from_bits_truncate(0o600),
        FchmodatFlags::NoFollowSymlink,
    )?;

    let meta = Metadata::from_path(&path, Options::default())?;
    let opts = Options {
        chmod: Some(parse_chmod("100644").unwrap()),
        ..Default::default()
    };
    meta.apply(&path, opts)?;

    let mode = fs::symlink_metadata(&path)?.permissions().mode();
    assert_eq!(normalize_mode(mode), 0o644);
    Ok(())
}
