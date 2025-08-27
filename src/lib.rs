use std::fs;
use std::path::Path;

use anyhow::Result;
use checksums::{rolling_checksum, strong_digest};
use protocol::{negotiate_version, Message, LATEST_VERSION};

/// Synchronize a single file from `src` to `dst` using the local protocol.
pub fn synchronize(src: &Path, dst: &Path) -> Result<()> {
    let data = fs::read(src)?;
    let weak = rolling_checksum(&data);
    let strong = strong_digest(&data);

    // sender side: build frames
    let mut frames = Vec::new();
    frames.push(Message::Version(LATEST_VERSION).to_frame(0));
    frames.push(Message::Data(data.clone()).to_frame(0));
    frames.push(Message::Done.to_frame(0));
    frames.push(Message::KeepAlive.to_frame(0));

    // receiver side: process frames
    let mut wrote = false;
    for f in frames {
        match Message::from_frame(f)? {
            Message::Version(v) => { negotiate_version(v).ok_or_else(|| anyhow::anyhow!("version"))?; }
            Message::Data(d) => {
                fs::write(dst, &d)?;
                assert_eq!(rolling_checksum(&d), weak);
                assert_eq!(strong_digest(&d), strong);
                wrote = true;
            }
            Message::Done => break,
            Message::KeepAlive => {},
        }
    }
    if !wrote {
        return Err(anyhow::anyhow!("no data written"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::io::Write;

    #[test]
    fn sync_local() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dst = dir.path().join("dst.txt");
        fs::File::create(&src).unwrap().write_all(b"hello world").unwrap();
        synchronize(&src, &dst).unwrap();
        let out = fs::read(&dst).unwrap();
        assert_eq!(out, b"hello world");
    }
}
