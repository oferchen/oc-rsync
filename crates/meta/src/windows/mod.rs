// crates/meta/src/windows/mod.rs
use filetime::{FileTime, set_file_times};
use std::fs;
use std::io;
use std::path::Path;

use crate::{Metadata, Options};

impl Metadata {
    pub fn from_path(path: &Path, opts: Options) -> io::Result<Self> {
        let meta = fs::metadata(path)?;
        let mut mode = 0o666;
        if meta.permissions().readonly() {
            mode &= !0o222;
        }
        let mtime = FileTime::from_last_modification_time(&meta);
        let atime = if opts.atimes {
            meta.accessed().ok().map(FileTime::from_system_time)
        } else {
            None
        };
        let crtime = if opts.crtimes {
            meta.created().ok().map(FileTime::from_system_time)
        } else {
            None
        };
        Ok(Metadata {
            uid: 0,
            gid: 0,
            mode,
            mtime,
            atime,
            crtime,
            #[cfg(feature = "acl")]
            acl: Vec::new(),
            #[cfg(feature = "acl")]
            default_acl: Vec::new(),
        })
    }

    pub fn apply(&self, path: &Path, opts: Options) -> io::Result<()> {
        if opts.perms || opts.executability {
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_readonly(self.mode & 0o222 == 0);
            fs::set_permissions(path, perms)?;
        }
        if opts.times {
            let atime = self.atime.unwrap_or(self.mtime);
            set_file_times(path, atime, self.mtime)?;
        }
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct HardLinks;

impl HardLinks {
    pub fn register(&mut self, _id: u64, _path: &Path) -> bool {
        false
    }

    pub fn finalize(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn hard_link_id(_dev: u64, _ino: u64) -> u64 {
    0
}

#[cfg(feature = "acl")]
pub fn read_acl(_path: &Path, _fake_super: bool) -> io::Result<(Vec<ACLEntry>, Vec<ACLEntry>)> {
    Ok((Vec::new(), Vec::new()))
}

#[cfg(feature = "acl")]
pub fn write_acl(
    _path: &Path,
    _acl: &[ACLEntry],
    _default_acl: Option<&[ACLEntry]>,
    _fake_super: bool,
    _super_user: bool,
) -> io::Result<()> {
    Ok(())
}

#[cfg(feature = "xattr")]
pub fn store_fake_super(_path: &Path, _uid: u32, _gid: u32, _mode: u32) {}
