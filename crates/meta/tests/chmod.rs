// crates/meta/tests/chmod.rs
use std::fs;
use std::os::unix::fs::PermissionsExt;

use meta::{Metadata, Options, normalize_mode, parse_chmod};
use nix::fcntl::AT_FDCWD;
use nix::sys::stat::{FchmodatFlags, Mode, fchmodat};
use tempfile::tempdir;

#[test]
fn chmod_numeric_mode_normalized() -> std::io::Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("file");
    fs::write(&path, b"test")?;

    fchmodat(
        AT_FDCWD,
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
