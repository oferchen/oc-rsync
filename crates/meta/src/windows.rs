// crates/meta/src/windows.rs
use filetime::{set_file_times, FileTime};
#[cfg(feature = "acl")]
use posix_acl::ACLEntry;
#[cfg(feature = "xattr")]
use std::ffi::{OsStr, OsString};
#[cfg(feature = "xattr")]
type XattrFilter = std::rc::Rc<dyn Fn(&OsStr) -> bool>;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
#[cfg(feature = "xattr")]
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodTarget {
    All,
    File,
    Dir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodOp {
    Add,
    Remove,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chmod {
    pub target: ChmodTarget,
    pub op: ChmodOp,
    pub mask: u32,
    pub bits: u32,
    pub conditional: bool,
}

#[derive(Clone, Default)]
pub struct Options {
    pub xattrs: bool,
    pub acl: bool,
    pub fake_super: bool,
    pub super_user: bool,
    pub numeric_ids: bool,
    pub chmod: Option<Vec<Chmod>>,
    pub owner: bool,
    pub group: bool,
    pub perms: bool,
    pub executability: bool,
    pub times: bool,
    pub atimes: bool,
    pub crtimes: bool,
    pub omit_dir_times: bool,
    pub omit_link_times: bool,
    pub uid_map: Option<Arc<dyn Fn(u32) -> u32 + Send + Sync>>,
    pub gid_map: Option<Arc<dyn Fn(u32) -> u32 + Send + Sync>>,
    #[cfg(feature = "xattr")]
    pub xattr_filter: Option<XattrFilter>,
    #[cfg(feature = "xattr")]
    pub xattr_filter_delete: Option<XattrFilter>,
}

impl fmt::Debug for Options {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Options")
            .field("xattrs", &self.xattrs)
            .field("acl", &self.acl)
            .field("fake_super", &self.fake_super)
            .field("super_user", &self.super_user)
            .field("numeric_ids", &self.numeric_ids)
            .field("chmod", &self.chmod)
            .field("owner", &self.owner)
            .field("group", &self.group)
            .field("perms", &self.perms)
            .field("executability", &self.executability)
            .field("times", &self.times)
            .field("atimes", &self.atimes)
            .field("crtimes", &self.crtimes)
            .field("omit_dir_times", &self.omit_dir_times)
            .field("omit_link_times", &self.omit_link_times)
            .field("uid_map", &self.uid_map.is_some())
            .field("gid_map", &self.gid_map.is_some())
            .field("xattr_filter", &{
                #[cfg(feature = "xattr")]
                {
                    self.xattr_filter.is_some()
                }
                #[cfg(not(feature = "xattr"))]
                {
                    false
                }
            })
            .field("xattr_filter_delete", &{
                #[cfg(feature = "xattr")]
                {
                    self.xattr_filter_delete.is_some()
                }
                #[cfg(not(feature = "xattr"))]
                {
                    false
                }
            })
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub mtime: FileTime,
    pub atime: Option<FileTime>,
    pub crtime: Option<FileTime>,
    #[cfg(feature = "xattr")]
    pub xattrs: Vec<(OsString, Vec<u8>)>,
    #[cfg(feature = "acl")]
    pub acl: Vec<ACLEntry>,
    #[cfg(feature = "acl")]
    pub default_acl: Vec<ACLEntry>,
}

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
            #[cfg(feature = "xattr")]
            xattrs: Vec::new(),
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
    _default_acl: &[ACLEntry],
    _fake_super: bool,
    _super_user: bool,
) -> io::Result<()> {
    Ok(())
}

#[cfg(feature = "xattr")]
pub fn store_fake_super(_path: &Path, _uid: u32, _gid: u32, _mode: u32) {}
